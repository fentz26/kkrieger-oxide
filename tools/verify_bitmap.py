#!/usr/bin/env python3
"""Generate exact MMX reference pixels in bounded x86 emulation.

Run with .reference/venv/bin/python after installing Unicorn 2.1.4 there.
Original function bodies are extracted unchanged from pinned Farbrausch source.
No emulator, original assembly, or C++ object is linked into the Rust runtime.
"""
import pathlib
import random
import struct
import subprocess
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32
from unicorn.x86_const import UC_X86_REG_ESP

root = pathlib.Path(__file__).resolve().parent.parent
ref = root / ".reference"
source = (ref / "fr_public/werkkzeug3_kkrieger/genbitmap.cpp").read_text(errors="replace")
start = source.index("void __stdcall Bitmap_Inner(")
end = source.index("GenBitmap * __stdcall Bitmap_Merge", start)
cpp = ref / "bitmap-inner.cpp"
fade = source[source.index("void Fade64("):source.index("void AddScale64(")]
# Clang's MS inline-asm reference lookup differs from MSVC's. Pointers have the
# same calling ABI here and make [r]/[c0]/[c1] refer to the argument slots as in
# MSVC. The assembly instructions remain unchanged.
fade = fade.replace("sU64 &r,sU64 &c0,sU64 &c1", "sU64 *r,sU64 *c0,sU64 *c1")
bilinear = source[source.index("void BilinearFilter("):source.index("static sInt GammaTable")]
context = "struct BilinearContext {sU64*Src;sInt XShift,XSize1,YSize1;unsigned XAm,YAm;};\n"
cpp.write_text("typedef unsigned long long sU64; typedef int sInt;\n" + context + source[start:end] + fade + bilinear)
obj = ref / "bitmap-inner.obj"
subprocess.run(["clang++", "-target", "i686-pc-windows-msvc", "-fms-extensions",
                "-fasm-blocks", "-fno-stack-protector", "-O2", "-msse2", "-c",
                str(cpp), "-o", str(obj)], check=True)

# Resolve this self-contained COFF object's section/symbol relocations without
# importing any Windows library or invoking a game executable natively.
blob = obj.read_bytes()
machine, count, _, symbols, n_symbols, opt_size, _ = struct.unpack_from("<HHIIIHH", blob)
assert machine == 0x14C
base = 0x400000
sections = []
for i in range(count):
    off = 20 + opt_size + i * 40
    _, _, size, raw, relocs, _, n_relocs, _, _ = struct.unpack_from("<IIIIIIHHI", blob, off + 8)
    assert size <= 4096
    sections.append((base + i * 4096, size, raw, relocs, n_relocs))
string_base = symbols + n_symbols * 18
symbol_values = []
entry = None
fade_entry = None
sample_entry = None
i = 0
while i < n_symbols:
    off = symbols + i * 18
    name = blob[off:off + 8]
    if name[:4] == b"\0" * 4:
        pos = string_base + struct.unpack_from("<I", name, 4)[0]
        name = blob[pos:].split(b"\0")[0]
    else:
        name = name.rstrip(b"\0")
    value, section, _, _, auxiliaries = struct.unpack_from("<IhHBB", blob, off + 8)
    address = sections[section - 1][0] + value if section > 0 else value
    symbol_values.append(address)
    if b"Bitmap_Inner" in name:
        entry = address
    if name.startswith(b"?Fade64@@"):
        fade_entry = address
    if name.startswith(b"?BilinearFilter@@"):
        sample_entry = address
    symbol_values.extend([0] * auxiliaries)
    i += 1 + auxiliaries
assert entry is not None
mu = Uc(UC_ARCH_X86, UC_MODE_32)
mu.mem_map(base, count * 4096)
for address, size, raw, relocs, n_relocs in sections:
    if size:
        mu.mem_write(address, blob[raw:raw + size])
    for i in range(n_relocs):
        offset, symbol, kind = struct.unpack_from("<IIH", blob, relocs + i * 10)
        location = address + offset
        addend = struct.unpack("<I", mu.mem_read(location, 4))[0]
        target = symbol_values[symbol]
        assert kind in (6, 20), f"unsupported relocation {kind}"
        value = addend + target - (location + 4 if kind == 20 else 0)
        mu.mem_write(location, struct.pack("<I", value & 0xFFFFFFFF))
stack, data, stop = 0x200000, 0x500000, 0x600000
mu.mem_map(stack, 0x10000)
mu.mem_map(data, 0x10000)
mu.mem_map(stop, 0x1000)
rng = random.Random(0x4B4B)
out = bytearray(struct.pack("<I", 40))
for full_words in (False, True):
    for mode in range(20):
        count = 32
        arrays = [struct.pack("<" + "H" * (count * 4),
                              *(rng.randrange(65536 if full_words else 32768) for _ in range(count * 4)))
                  for _ in range(3)]
        d, s, x = data, data + 4096, data + 8192
        for address, pixels in zip((d, s, x), arrays):
            mu.mem_write(address, pixels)
        esp = stack + 0xFF00
        mu.mem_write(esp, struct.pack("<IIIIII", stop, d, s, count, mode, x))
        mu.reg_write(UC_X86_REG_ESP, esp)
        mu.emu_start(entry, stop, timeout=1_000_000, count=100_000)
        out.extend(struct.pack("<II", mode, count))
        for pixels in arrays:
            out.extend(pixels)
        out.extend(mu.mem_read(d, count * 8))
destination = root / "tests/fixtures/bitmap-inner.bin"
destination.parent.mkdir(exist_ok=True)
destination.write_bytes(out)
print(destination)
assert sample_entry is not None
width, height, cases = 8, 4, 1000
sample_rng = random.Random(0xB11)
image = struct.pack("<" + "H" * (width * height * 4),
                    *(sample_rng.randrange(65536) for _ in range(width * height * 4)))
mu.mem_write(data, image)
out = bytearray(struct.pack("<III", cases, width, height) + image)
for i in range(cases):
    edges = [0, -1, 65535, 65536, -65536, 7 * 65536, 8 * 65536, 0x7FFFFFFF, -0x80000000]
    u = edges[i] if i < len(edges) else sample_rng.randrange(-32 * 65536, 32 * 65536)
    v = edges[-i - 1] if i < len(edges) else sample_rng.randrange(-16 * 65536, 16 * 65536)
    border = i % 4
    context = struct.pack("<IIIIII", data, 6, (width - 1) * 8, height - 1,
                          0xFFFFFFF8 if border & 1 else (width - 1) * 8,
                          0xFFFFFFFF if border & 2 else height - 1)
    mu.mem_write(data + 12288, context)
    esp = stack + 0xFF00
    mu.mem_write(esp, struct.pack("<IIIII", stop, data + 12288, data + 4096,
                                 u & 0xFFFFFFFF, v & 0xFFFFFFFF))
    mu.reg_write(UC_X86_REG_ESP, esp)
    mu.emu_start(sample_entry, stop, timeout=1_000_000, count=100_000)
    out.extend(struct.pack("<iiI", u, v, border) + mu.mem_read(data + 4096, 8))
destination = root / "tests/fixtures/bitmap-bilinear.bin"
destination.write_bytes(out)
print(destination)
assert fade_entry is not None
out = bytearray(struct.pack("<I", 512))
for i in range(512):
    colors = [struct.pack("<4H", *(rng.randrange(65536) for _ in range(4))) for _ in range(2)]
    fades = [0, 1, 32767, 32768, 65535, 65536, -1, 100000]
    fade = fades[i] if i < len(fades) else rng.randrange(-100000, 200000)
    mu.mem_write(data + 4096, colors[0])
    mu.mem_write(data + 8192, colors[1])
    esp = stack + 0xFF00
    mu.mem_write(esp, struct.pack("<IIIII", stop, data, data + 4096, data + 8192, fade & 0xFFFFFFFF))
    mu.reg_write(UC_X86_REG_ESP, esp)
    mu.emu_start(fade_entry, stop, timeout=1_000_000, count=100_000)
    out.extend(colors[0] + colors[1] + struct.pack("<i", fade) + mu.mem_read(data, 8))
destination = root / "tests/fixtures/bitmap-fade.bin"
destination.write_bytes(out)
print(destination)
