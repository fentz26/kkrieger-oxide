"""Scalar lane semantics for executing unchanged MMX-intrinsic source on ARM.

Only used by the offline reference harness. Pixel primitives and bilinear
sampling are additionally cross-checked against emulated original MMX.
"""
MMX = r'''
using __m64=uint64_t;using __int64=int64_t;
uint16_t lane16(uint64_t a,int i){return a>>(i*16);}
uint32_t lane32(uint64_t a,int i){return a>>(i*32);}
uint64_t pack16(int64_t a,int64_t b,int64_t c,int64_t d){return uint64_t(uint16_t(a))|(uint64_t(uint16_t(b))<<16)|(uint64_t(uint16_t(c))<<32)|(uint64_t(uint16_t(d))<<48);}
uint64_t pack32(uint32_t a,uint32_t b){return uint64_t(a)|(uint64_t(b)<<32);}
template<class F>uint64_t lanes16(uint64_t a,uint64_t b,F f){return pack16(f(lane16(a,0),lane16(b,0)),f(lane16(a,1),lane16(b,1)),f(lane16(a,2),lane16(b,2)),f(lane16(a,3),lane16(b,3)));}
template<class F>uint64_t lanes32(uint64_t a,uint64_t b,F f){return pack32(f(lane32(a,0),lane32(b,0)),f(lane32(a,1),lane32(b,1)));}
__m64 _mm_set_pi16(int a,int b,int c,int d){return pack16(d,c,b,a);}
__m64 _mm_set_pi32(int a,int b){return pack32(b,a);}
__m64 _mm_unpacklo_pi16(__m64 a,__m64 b){return pack16(lane16(a,0),lane16(b,0),lane16(a,1),lane16(b,1));}
__m64 _mm_unpackhi_pi16(__m64 a,__m64 b){return pack16(lane16(a,2),lane16(b,2),lane16(a,3),lane16(b,3));}
__m64 _mm_madd_pi16(__m64 a,__m64 b){
 auto pair=[&](int i){return uint32_t(int64_t(int16_t(lane16(a,i)))*int16_t(lane16(b,i))+int64_t(int16_t(lane16(a,i+1)))*int16_t(lane16(b,i+1)));};
 return pack32(pair(0),pair(2));
}
__m64 _mm_add_pi32(__m64 a,__m64 b){return lanes32(a,b,[](uint32_t x,uint32_t y){return x+y;});}
__m64 _mm_sub_pi32(__m64 a,__m64 b){return lanes32(a,b,[](uint32_t x,uint32_t y){return x-y;});}
__m64 _mm_packs_pi32(__m64 a,__m64 b){return pack16(sat(int32_t(lane32(a,0))),sat(int32_t(lane32(a,1))),sat(int32_t(lane32(b,0))),sat(int32_t(lane32(b,1))));}
__m64 _mm_srli_pi32(__m64 a,int c){return lanes32(a,0,[&](uint32_t x,uint32_t){return x>>c;});}
__m64 _mm_srai_pi32(__m64 a,int c){return lanes32(a,0,[&](uint32_t x,uint32_t){return uint32_t(int32_t(x)>>c);});}
__m64 _mm_slli_pi32(__m64 a,int c){return lanes32(a,0,[&](uint32_t x,uint32_t){return x<<c;});}
__m64 _mm_srli_pi16(__m64 a,int c){return lanes16(a,0,[&](uint16_t x,uint16_t){return x>>c;});}
__m64 _mm_cmpgt_pi32(__m64 a,__m64 b){return lanes32(a,b,[](uint32_t x,uint32_t y){return int32_t(x)>int32_t(y)?0xffffffffu:0u;});}
__m64 _mm_adds_pu16(__m64 a,__m64 b){return lanes16(a,b,[](uint16_t x,uint16_t y){return std::min(65535,int(x)+y);});}
__m64 _mm_mullo_pi16(__m64 a,__m64 b){return lanes16(a,b,[](uint16_t x,uint16_t y){return uint32_t(x)*y;});}
__m64 _mm_mulhi_pu16(__m64 a,__m64 b){return lanes16(a,b,[](uint16_t x,uint16_t y){return uint32_t(x)*y>>16;});}
__m64 _mm_add_pi16(__m64 a,__m64 b){return lanes16(a,b,[](uint16_t x,uint16_t y){return x+y;});}
__m64 _mm_subs_pi16(__m64 a,__m64 b){return lanes16(a,b,[](uint16_t x,uint16_t y){return sat(int(int16_t(x))-int16_t(y));});}

uint16_t highmul(uint16_t a,uint16_t b){return int32_t(int16_t(a))*int16_t(b)>>16;}
uint16_t twice(uint16_t a,uint16_t b){return uint16_t(highmul(a,b))<<1;}
uint16_t sadd(uint16_t a,uint16_t b){return sat(int(int16_t(a))+int16_t(b));}
uint16_t ssub(uint16_t a,uint16_t b){return sat(int(int16_t(a))-int16_t(b));}
uint16_t usub(uint16_t a,uint16_t b){return a>b?a-b:0;}
uint16_t gray64(uint64_t a){uint16_t bg=lane16(a,0)+lane16(a,1),rg=lane16(a,2)+lane16(a,1);return uint16_t((bg>>1)+(rg>>1))>>1;}
void Bitmap_Inner(uint64_t*d,uint64_t*s,int count,int mode,uint64_t*x=nullptr){
 for(int k=0;k<count;k++){
  uint64_t a=d[k],b=(mode>=5&&mode<=10)||mode==15||mode==19?s[0]:s[k],base=x[k],out=0;
  for(int i=0;i<4;i++){
   uint16_t v=0,dd=lane16(a,i),ss=lane16(b,i),xx=lane16(base,i);
   switch(mode){
   case 0:case 6:v=sadd(xx,ss);break;
   case 1:case 7:v=usub(xx,ss);break;
   case 2:case 5:v=twice(xx,ss);break;
   case 3:v=uint16_t(xx-ss+32767)>>1;break;
   case 4:v=i==3?gray64(b):xx&32767;break;
   case 8:v=i==3?32767:gray64(base);break;
   case 9:v=xx^32767;break;
   case 10:v=sat(uint32_t(int32_t(int16_t(xx))*int16_t(ss))>>11);break;
   case 11:v=uint16_t(sadd(highmul(dd,ss),highmul(ssub(32767,dd),xx)))<<1;break;
   case 12:{uint16_t mask=int16_t(ss)>16383?32767:0;v=uint16_t(highmul(ss^mask,xx^mask)<<2)^mask;break;}
   case 13:v=usub(ss,xx);break;
   case 14:v=uint16_t(sadd(highmul(dd,twice(ss,xx)),highmul(ssub(32767,dd),xx)))<<1;break;
   case 15:v=std::max(0,int(int16_t(sadd(sat(int32_t(int16_t(usub(xx,dd)))*int16_t(ss)>>11),xx))));break;
   case 16:{uint16_t t=ss<<1,mask=int16_t(t)>>15;t&=32767;v=uint16_t((twice(xx,t)^mask)+((t+xx)&mask));break;}
   case 17:v=std::max(0,int(int16_t(sadd(twice(ssub(ss,xx),lane16(b,3)),xx))));break;
   case 18:v=twice(xx^32767,ss^32767)^32767;break;
   case 19:v=std::max(0,int(int16_t(sadd(twice(xx,ssub(lane16(s[1],i),ss)),ss))));break;
   default:abort();
   }out|=uint64_t(v)<<(i*16);
  }d[k]=out;
 }
}
void AddScale64(uint64_t&r,uint64_t&a,uint64_t&b,int fade){auto aa=a,bb=b;uint64_t out=0;auto weight=uint16_t(sat(fade>>1));for(int i=0;i<4;i++)out|=uint64_t(sadd(lane16(aa,i),twice(lane16(bb,i),weight)))<<(i*16);r=out;}
struct BilinearContext{uint64_t*Src;int XShift,XSize1,YSize1;uint32_t XAm,YAm;};
void BilinearSetup(BilinearContext*c,uint64_t*src,int w,int h,int border){c->Src=src;c->XShift=sGetPower2(w)+3;c->XSize1=(w-1)*8;c->YSize1=h-1;c->XAm=border;}
void BilinearFilter(BilinearContext*c,uint64_t*r,int u,int v){
 int width=(c->XSize1/8)+1,height=c->YSize1+1;unsigned border=c->XAm;
 auto coordinate=[](int i,int size,bool clamp){return clamp?std::clamp(i,0,size-1):int(uint32_t(i)&(size-1));};
 int x=u>>16,y=v>>16,x0=coordinate(x,width,border&1),x1=coordinate(x+1,width,border&1),y0=coordinate(y,height,border&2),y1=coordinate(y+1,height,border&2);
 uint64_t a=c->Src[y0*width+x0],b=c->Src[y0*width+x1],cc=c->Src[y1*width+x0],d=c->Src[y1*width+x1],out=0;
 uint16_t fx=(uint16_t(u)>>1),fy=(uint16_t(v)>>1);
 for(int i=0;i<4;i++){
  uint16_t top=lane16(a,i)+twice(uint16_t(lane16(b,i)-lane16(a,i)),fx);
  uint16_t bottom=lane16(cc,i)+twice(uint16_t(lane16(d,i)-lane16(cc,i)),fx);
  uint16_t value=top+32768+twice(uint16_t(bottom-top),fy);value=usub(value,32768);
  out|=uint64_t(value)<<(i*16);
 }*r=out;
}
'''
