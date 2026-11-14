//! GLSL translation of the original Material11 vs.1.1/ps.1.1 programs.
//! Register uniforms retain the original Set() layout. Lighting lookup textures
//! are supplied by the renderer; attenuation is a lossless 32-slice 2D atlas.
use super::State;
use std::fmt::Write;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Image {
    Texture(usize),
    Normalizer,
    Attenuation,
}
#[derive(Debug)]
pub struct Sources {
    pub vertex: String,
    pub fragment: String,
    pub images: Vec<(String, Image)>,
}

fn alpha(index: u32) -> Result<String, &'static str> {
    Ok(match index {
        0 | 3 => "1.0".into(),
        1 => "r1.z".into(),
        2 => "r0.w".into(),
        4..=7 => format!("t{}.w", index - 4),
        8..=11 => format!("pc[{}].w", index - 8),
        12 => "0.5".into(),
        13 => "0.0".into(),
        _ => return Err("invalid original alpha source"),
    })
}
fn color(index: usize) -> Result<String, &'static str> {
    Ok(match index {
        0 => "r0.xyz".into(),
        1 => "pc[0].xyz".into(),
        2 => "pc[1].xyz".into(),
        3 => "t0.xyz".into(),
        4 => "t1.xyz".into(),
        6 => "vec3(r0.w)".into(),
        7 => "pc[2].xyz".into(),
        8 => "pc[3].xyz".into(),
        9 => "t2.xyz".into(),
        10 => "t3.xyz".into(),
        11 => "v_color.xyz".into(),
        _ => return Err("invalid original color source"),
    })
}
impl State {
    pub fn shader_sources(&self) -> Result<Sources, &'static str> {
        let w = &self.words;
        let mut textures = self.textures;
        // Compile() removes the unsupported bump-detail stage before codegen.
        if w[1] & 1 != 0 {
            textures[0] = None;
        }
        let require_alpha = |a: u32| -> Result<(), &'static str> {
            if (4..=7).contains(&a) && textures[(a - 4) as usize].is_none() {
                return Err("original shader references an unbound alpha texture");
            }
            Ok(())
        };
        if w[2] & 7 != 5 {
            for i in 0..13 {
                if w[8 + i] & 0xf0 == 0 {
                    continue;
                }
                let texture = match i {
                    3 => Some(0),
                    4 => Some(1),
                    9 => Some(2),
                    10 => Some(3),
                    _ => None,
                };
                if texture.is_some_and(|n| textures[n].is_none()) {
                    return Err("original shader references an unbound color texture");
                }
                require_alpha(w[8 + i] & 15)?;
            }
            require_alpha(w[23] & 15)?;
            if (w[23] >> 4) & 15 == 14 {
                if textures.iter().filter(|t| t.is_some()).count() < 2 {
                    return Err("original mix01 shader requires two textures");
                }
            } else {
                require_alpha((w[23] >> 4) & 15)?;
            }
        }
        let lighting = w[2] & 7 == 5;
        let specular = w[1] & 0x40 == 0;
        let mut vertex = String::from(
            "#version 100\nprecision highp float;\nuniform vec4 vc[24];\nattribute vec4 position;\nattribute vec2 uv0;\nattribute vec2 uv1;\nattribute vec3 normal;\nattribute vec3 tangent;\nattribute vec4 color;\nvarying vec4 v_color;\nvarying float v_fog;\n",
        );
        let mut fragment = String::from(
            "#version 100\nprecision highp float;\nuniform vec4 pc[8];\nuniform vec4 fog_color;\nvarying vec4 v_color;\nvarying float v_fog;\n",
        );
        let mut images = Vec::new();
        for (i, t) in textures.iter().enumerate() {
            if let Some(id) = t {
                writeln!(vertex, "varying vec2 v_uv{i};").unwrap();
                writeln!(
                    fragment,
                    "varying vec2 v_uv{i};\nuniform sampler2D image{i};"
                )
                .unwrap();
                images.push((format!("image{i}"), Image::Texture(*id)));
            }
        }
        if lighting {
            vertex.push_str("varying vec3 v_light;\nvarying vec3 v_attn;\n");
            fragment.push_str("varying vec3 v_light;\nvarying vec3 v_attn;\nuniform samplerCube normalizer;\nuniform sampler2D attenuation;\n");
            images.push(("normalizer".into(), Image::Normalizer));
            images.push(("attenuation".into(), Image::Attenuation));
            if specular {
                vertex.push_str("varying vec3 v_half;\n");
                fragment.push_str("varying vec3 v_half;\n");
            }
            fragment.push_str(ATTENUATION);
        }
        vertex.push_str("void main(){\nvec4 modelPos=position;\n");
        if w[0] & 0x40000 != 0 {
            vertex.push_str("modelPos=vc[4]*(position.w-1.0)+position;\n");
        }
        vertex.push_str("gl_Position=vec4(dot(modelPos,vc[0]),dot(modelPos,vc[1]),dot(modelPos,vc[2]),dot(modelPos,vc[3]));\n");
        // D3D clip depth [0,w] maps to OpenGL [-w,w].
        vertex.push_str("gl_Position.z=gl_Position.z*2.0-gl_Position.w;\nv_color=color;\nv_fog=1.0;\nvec3 N=normal;vec3 T=tangent;vec3 B=cross(N,T);\n");
        if w[0] & 0x40 != 0 {
            vertex.push_str("v_fog=dot(modelPos,vc[20]);\n");
        }
        for (i, t) in textures.iter().enumerate() {
            if t.is_none() {
                continue;
            }
            vertex.push_str("{\n");
            match (w[4 + i] >> 4) & 3 {
                0 => vertex.push_str("vec4 inUV=vec4(uv0,0.0,1.0);\n"),
                1 => vertex.push_str("vec4 inUV=vec4(uv1,0.0,1.0);\n"),
                _ => return Err("unsupported original UV input"),
            }
            if i == 2 {
                match (w[1]>>4)&3 {
                1=>vertex.push_str("vec2 en=vec2(dot(N,vc[12].xyz),dot(N,vc[13].xyz));inUV.xy=en*vec2(0.5,-0.5)+vec2(0.5);\n"),
                2=>vertex.push_str("vec3 ep=normalize(vec3(dot(modelPos,vc[12]),dot(modelPos,vc[13]),dot(modelPos,vc[14])));vec3 en=vec3(dot(N,vc[12].xyz),dot(N,vc[13].xyz),dot(N,vc[14].xyz));inUV.xyz=en*(dot(ep,en)*2.0/dot(en,en))-ep;inUV.xy=inUV.xy*vec2(0.5,-0.5)+vec2(0.5);\n"),_=>{}
            }
            }
            if i == 3 {
                match (w[1]>>8)&15 {1=>vertex.push_str("inUV=modelPos;\n"),2|3=>vertex.push_str("inUV=vec4(dot(modelPos,vc[16]),dot(modelPos,vc[17]),dot(modelPos,vc[18]),1.0);\n"),_=>{}}
            }
            let expression = match (w[4 + i] >> 12) & 15 {
                0 => "inUV.xy".into(),
                1 => format!("inUV.xy*vc[7].{}", ["x", "y", "z", "w"][i]),
                2 => "vec2(dot(inUV,vc[8]),dot(inUV,vc[9]))".into(),
                3 => "vec2(dot(inUV,vc[10]),dot(inUV,vc[11]))".into(),
                _ => return Err("original texture transform has undefined outputs"),
            };
            writeln!(vertex, "v_uv{i}={expression};\n}}").unwrap();
        }
        if lighting {
            vertex.push_str(
                "vec3 L=vc[4].xyz-modelPos.xyz;v_light=vec3(dot(L,N),dot(L,B),dot(L,T));\n",
            );
            if specular {
                vertex.push_str("vec3 H=normalize(L)+normalize(vc[6].xyz-modelPos.xyz);v_half=vec3(dot(H,N),dot(H,B),dot(H,T));\n");
            }
            vertex.push_str("v_attn=modelPos.xyz*vc[5].w+vc[5].xyz;\n");
        }
        vertex.push_str("}\n");
        fragment.push_str("void main(){\nvec4 r0=vec4(0.0),r1=vec4(0.0);\n");
        for (i, texture) in textures.iter().enumerate() {
            if texture.is_some() {
                writeln!(fragment, "vec4 t{i}=texture2D(image{i},v_uv{i});").unwrap();
            } else {
                writeln!(fragment, "vec4 t{i}=vec4(0.0);").unwrap();
            }
        }
        if lighting {
            // The source reads the first bound texture register as its normal.
            let normal = if textures[1].is_some() {
                format!(
                    "t{}.xyz",
                    textures.iter().position(Option::is_some).unwrap()
                )
            } else {
                "vec3(1.0,0.0,0.0)".into()
            };
            writeln!(fragment,"vec3 bump={normal};vec4 att=sample_attenuation(v_attn);vec3 L=textureCube(normalizer,v_light).xyz*2.0-1.0;").unwrap();
            if !specular {
                fragment.push_str(
                    "r0.xyz=vec3(clamp(dot(L,bump),0.0,1.0))*att.xyz*pc[0].xyz;r0.w=0.0;\n",
                );
            } else {
                fragment.push_str("vec3 H=textureCube(normalizer,v_half).xyz*2.0-1.0;float spec=clamp(dot(H,bump),0.0,1.0);float diff=clamp(dot(L,bump),0.0,1.0);spec=clamp((spec*pc[1].z+pc[1].w)*4.0,0.0,1.0);spec=spec*spec*2.0;spec=spec*spec*2.0;spec=clamp(spec*spec*2.0,0.0,1.0);r0.xyz=pc[0].xyz*att.xyz*diff;r0.w=pc[0].w*att.w*spec;\n");
                if w[1] & 0x80 != 0 {
                    let gloss = textures
                        .iter()
                        .position(Option::is_some)
                        .map(|i| format!("t{i}.w"))
                        .unwrap_or_else(|| "0.0".into());
                    writeln!(fragment, "r0.w*={gloss};").unwrap();
                }
            }
        } else {
            for i in 0..13 {
                let flags = w[8 + i];
                let op = (flags >> 4) & 15;
                if op == 0 {
                    continue;
                }
                let mut src = color(i)?;
                let a = alpha(flags & 15)?;
                if flags & 15 != 0 && op != 9 {
                    src = format!("({src}*{a})");
                }
                let expression = match op {
                    1 => src,
                    2 => format!("r0.xyz+{src}"),
                    3 => format!("r0.xyz-{src}"),
                    4 => format!("r0.xyz*{src}"),
                    5 => format!("r0.xyz*{src}*2.0"),
                    6 => format!("r0.xyz*{src}*4.0"),
                    7 => format!("r0.xyz*{src}*8.0"),
                    8 => format!("r0.xyz*(vec3(1.0)-{src})+{src}"),
                    9 => format!("mix({src},r0.xyz,{a})"),
                    10 => format!("{src}-r0.xyz"),
                    11 => format!("vec3(dot(r0.xyz,{src}))"),
                    12 => format!("clamp(r0.xyz*{src},0.0,1.0)"),
                    _ => return Err("invalid original combiner operation"),
                };
                writeln!(fragment, "r0.xyz={expression};").unwrap();
            }
            let flags = w[23];
            let a = alpha(flags & 15)?;
            let a = if flags & 0x100 != 0 {
                format!("(1.0-{a})")
            } else {
                a
            };
            let b = alpha((flags >> 4) & 15);
            let result = if (flags >> 4) & 15 == 14 {
                {
                    let bound: Vec<_> = textures
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| t.is_some())
                        .map(|(i, _)| i)
                        .collect();
                    format!("t{}.w*pc[0].w+t{}.w*pc[0].w", bound[0], bound[1])
                }
            } else if (flags >> 4) & 15 != 0 {
                let b = b?;
                let b = if flags & 0x200 != 0 {
                    format!("(1.0-{b})")
                } else {
                    b
                };
                format!("{a}*{b}")
            } else {
                a
            };
            writeln!(fragment, "r0.w={result};").unwrap();
        }
        if w[0] & 1 != 0 {
            fragment.push_str("if(r0.w<=128.0/255.0)discard;\n");
        }
        if w[0] & 0x40 != 0 {
            fragment.push_str("r0.xyz=mix(fog_color.xyz,r0.xyz,clamp(v_fog,0.0,1.0));\n");
        }
        fragment.push_str("gl_FragColor=r0;\n}\n");
        Ok(Sources {
            vertex,
            fragment,
            images,
        })
    }
}
const ATTENUATION: &str = "vec4 sample_attenuation(vec3 p){\nvec3 q=clamp(p*32.0-0.5,0.0,31.0);float z=floor(q.z);vec2 uv=(q.xy+0.5)/vec2(32.0,1024.0);uv.y+=z/32.0;vec4 a=texture2D(attenuation,uv);uv.y+=min(1.0,31.0-z)/32.0;return mix(a,texture2D(attenuation,uv),q.z-z);\n}\n";
