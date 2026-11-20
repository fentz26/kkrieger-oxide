#!/usr/bin/env python3
"""Run released x86 Bitmap_Text through native Windows GDI, save pixels/UVs.

Requires tools/prepare_beta_reference.py and Unicorn 2.1.4. GDI handles are
translated between the 32-bit emulator and this process. Only allocation and
bitmap ownership checks are substituted. Original FontBegin, layout, width,
print, bitmap readback, downsample, color conversion and MMX blend execute.
GdiFlush is an observation barrier before native readback. Captures stay local;
no installed font data or rendered font pixels are redistributed.
"""
import ctypes as C
import hashlib
import pathlib
import struct
import sys
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_FPCW

ROOT = pathlib.Path(__file__).resolve().parent.parent
IMAGE_SHA = '148387146f40be8aed5212e361d27c8af1020675fd68ef0ef257d71d87931a42'
BASE, ENV, BITMAP, TEXT, FONT = 0x7c0000, 0x3000000, 0x3010000, 0x3011000, 0x3012000
SYSTEM, STACK, CPU, PIXELS, SERVICES, RETURN = 0x3100000, 0x303f000, 0x3800000, 0x3a00000, 0x5000000, 0x5000f00

def pack(*values):
    return struct.pack('<' + 'I' * len(values), *(v & 0xffffffff for v in values))

def fword(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]

class Gdi:
    def __init__(self):
        if sys.platform != 'win32':
            raise SystemExit('Native GDI capture requires Windows')
        self.dll = C.WinDLL('gdi32')
        self.user = C.WinDLL('user32')
        P, I, U = C.c_void_p, C.c_int, C.c_uint
        signatures = {
            'CreateCompatibleDC': (P, [P]), 'DeleteDC': (I, [P]),
            'CreateCompatibleBitmap': (P, [P, I, I]),
            'SelectObject': (P, [P, P]), 'DeleteObject': (I, [P]),
            'GetCurrentObject': (P, [P, U]), 'SetBkMode': (I, [P, I]),
            'SetTextColor': (U, [P, U]), 'GdiFlush': (I, []),
            'CreateFontA': (P, [I, I, I, I, I, U, U, U, U, U, U, U, U, C.c_char_p]),
            'GetTextMetricsA': (I, [P, P]),
            'GetTextExtentPoint32A': (I, [P, C.c_char_p, I, P]),
            'ExtTextOutA': (I, [P, I, I, U, P, C.c_char_p, U, P]),
            'SetDIBits': (I, [P, P, U, U, P, P, U]),
            'GetDIBits': (I, [P, P, U, U, P, P, U]),
        }
        for name, (result, args) in signatures.items():
            fn = getattr(self.dll, name)
            fn.restype, fn.argtypes = result, args
        self.user.GetDC.restype, self.user.GetDC.argtypes = P, [P]
        self.user.ReleaseDC.restype, self.user.ReleaseDC.argtypes = I, [P, P]
        self.handles = {0: None}
        self.next_handle = 1
        self.line_height = 0
        self.settings = []

    def handle(self, real):
        if not real:
            return 0
        if real == C.c_void_p(-1).value:
            return 0xffffffff
        for key, value in self.handles.items():
            if real == value:
                return key
        key = self.next_handle
        self.next_handle += 1
        self.handles[key] = real
        return key

    def start(self, u):
        self.screen = self.user.GetDC(None)
        self.memory = self.dll.CreateCompatibleDC(self.screen)
        self.placeholder = self.dll.CreateCompatibleBitmap(self.screen, 16, 16)
        assert self.screen and self.memory and self.placeholder
        self.old_bitmap = self.dll.SelectObject(self.memory, self.placeholder)
        self.old_screen_font = self.dll.GetCurrentObject(self.screen, 6)
        self.old_memory_font = self.dll.GetCurrentObject(self.memory, 6)
        for address, value in [(0x83f880, self.screen), (0x83f874, self.memory), (0x83f8bc, self.placeholder)]:
            u.mem_write(address, pack(self.handle(value)))

    def stop(self, u):
        # Released FontBitmap restores the placeholder and deletes its DDB.
        self.dll.GdiFlush()
        self.dll.SelectObject(self.screen, self.old_screen_font)
        self.dll.SelectObject(self.memory, self.old_memory_font)
        font = struct.unpack('<I', u.mem_read(0x83f8a0, 4))[0]
        if font:
            assert self.dll.DeleteObject(self.handles[font])
        self.dll.SelectObject(self.memory, self.old_bitmap)
        assert self.dll.DeleteObject(self.placeholder)
        assert self.dll.DeleteDC(self.memory)
        assert self.user.ReleaseDC(None, self.screen)

    def call(self, name, args, u):
        g = self.dll
        h = lambda i: self.handles[args[i]]
        signed = lambda value: C.c_int(value).value
        if name == 'CreateCompatibleBitmap':
            return self.handle(g.CreateCompatibleBitmap(h(0), signed(args[1]), signed(args[2])))
        if name == 'SelectObject':
            return self.handle(g.SelectObject(h(0), h(1)))
        if name == 'DeleteObject':
            return g.DeleteObject(h(0))
        if name == 'CreateFontA':
            self.settings.append(args[:13])
            return self.handle(g.CreateFontA(*(signed(v) for v in args[:13]), read_string(u, args[13])))
        if name == 'SetBkMode':
            return g.SetBkMode(h(0), signed(args[1]))
        if name == 'SetTextColor':
            return g.SetTextColor(h(0), args[1])
        if name == 'GetTextMetricsA':
            out = C.create_string_buffer(56)
            result = g.GetTextMetricsA(h(0), out)
            u.mem_write(args[1], out.raw)
            self.line_height = struct.unpack_from('<i', out.raw)[0]
            assert result and self.line_height > 0
            return result
        if name == 'GetTextExtentPoint32A':
            out = C.create_string_buffer(8)
            result = g.GetTextExtentPoint32A(h(0), bytes(u.mem_read(args[1], args[2])), args[2], out)
            u.mem_write(args[3], out.raw)
            assert result
            return result
        if name == 'ExtTextOutA':
            assert args[3] == args[4] == args[7] == 0
            result = g.ExtTextOutA(h(0), signed(args[1]), signed(args[2]), 0, None,
                                  bytes(u.mem_read(args[5], args[6])), args[6], None)
            assert result
            return result
        if name in ['SetDIBits', 'GetDIBits']:
            assert args[2] == args[6] == 0
            info = C.create_string_buffer(bytes(u.mem_read(args[5], 44)), 44)
            width, height = struct.unpack_from('<ii', info.raw, 4)
            count = width * abs(height) * 4
            assert 0 < count <= 0x200000
            if name == 'SetDIBits':
                pixels = C.create_string_buffer(bytes(u.mem_read(args[4], count)), count)
                result = g.SetDIBits(h(0), h(1), 0, args[3], pixels, info, 0)
            else:
                assert g.GdiFlush()
                pixels = C.create_string_buffer(count)
                result = g.GetDIBits(h(0), h(1), 0, args[3], pixels, info, 0)
                u.mem_write(args[4], pixels.raw)
            assert result == args[3], (name, result, args[3])
            return result
        raise AssertionError(name)

def read_string(u, pointer):
    result = bytearray()
    while (byte := u.mem_read(pointer, 1)[0]):
        result.append(byte)
        pointer += 1
        assert len(result) < 4096
    return bytes(result)

def main():
    image = (ROOT / '.reference/beta-image.bin').read_bytes()
    assert hashlib.sha256(image).hexdigest() == IMAGE_SHA
    u = Uc(UC_ARCH_X86, UC_MODE_32)
    u.mem_map(BASE, (len(image) + 4095) & ~4095)
    u.mem_write(BASE, image)
    u.mem_map(ENV, 0x40000)
    u.mem_map(SYSTEM, 0x300000)
    u.mem_map(CPU, 0x200000)
    u.mem_map(PIXELS, 0x40000)
    u.mem_map(SERVICES, 4096)
    api = Gdi()
    services = [
        (0x81f004, 'GetDIBits', 7), (0x81f008, 'ExtTextOutA', 8),
        (0x81f00c, 'GetTextExtentPoint32A', 4), (0x81f010, 'CreateCompatibleBitmap', 3),
        (0x81f014, 'SelectObject', 2), (0x81f018, 'SetDIBits', 7),
        (0x81f01c, 'CreateFontA', 14), (0x81f020, 'SetBkMode', 2),
        (0x81f024, 'SetTextColor', 2), (0x81f028, 'GetTextMetricsA', 2),
        (0x81f02c, 'DeleteObject', 1),
    ]
    addresses = {}
    for index, (iat, name, count) in enumerate(services):
        address = SERVICES + index * 16
        addresses[address] = (name, count)
        u.mem_write(iat, pack(address))
    read = lambda p: struct.unpack('<I', u.mem_read(p, 4))[0]
    def ret(pop, value=0):
        sp = u.reg_read(UC_X86_REG_ESP)
        u.reg_write(UC_X86_REG_EAX, value & 0xffffffff)
        u.reg_write(UC_X86_REG_EIP, read(sp))
        u.reg_write(UC_X86_REG_ESP, sp + 4 + pop)
    def hook(u, address, size, user):
        if address == 0x801000:  # operator new; original FontBegin zeros memory
            assert read(u.reg_read(UC_X86_REG_ESP) + 4) <= 0x200000
            ret(0, CPU)
        elif address in [0x801017, 0x80a608]:
            ret(0)
        else:
            name, count = addresses[address]
            sp = u.reg_read(UC_X86_REG_ESP)
            args = [read(sp + 4 + i * 4) for i in range(count)]
            result = api.call(name, args, u)
            ret(count * 4, result)
    for address in [0x801000, 0x801017, 0x80a608, *addresses]:
        u.hook_add(UC_HOOK_CODE, hook, begin=address, end=address)
    output = bytearray(pack(24))
    for case in range(24):
        side = 32 if case % 2 else 64
        font_id = (case // 2) % 2
        font = [b'Arial', b'Times New Roman'][font_id]
        flags = 0x10 if case % 3 else case % 2
        text = [b'a-zA-Z0-9.,-:;><! ', b'a-c', b'--!', b'Centered\ntext'][case % 4]
        params = [fword((case % 3 - 1) * .03125), fword((case % 5 - 2) * .03125),
                  fword([.099853515625, .125, .1875][case % 3]), fword(0),
                  [0xffffffff, 0xff804020, 0x7f70c030][case % 3], flags,
                  [0, 1, 8][case % 3], [163, fword(.003), fword(-.002)][case % 3]]
        u.mem_write(ENV, bytes(0x10000))
        u.mem_write(BITMAP, bytes(64))
        u.mem_write(BITMAP + 0xc, pack(PIXELS, side, side))
        # Original color conversion for a 0xff102030 background, low to high BGRA.
        background = struct.pack('<4H', *(((v << 7) | (v >> 1)) & 0x7fff for v in [0x30, 0x20, 0x10, 0xff]))
        u.mem_write(PIXELS, background * (side * side))
        u.mem_write(TEXT, text + b'\0')
        u.mem_write(FONT, font + b'\0')
        u.mem_write(0x83f868, pack(SYSTEM))
        api.start(u)
        try:
            u.mem_write(STACK, pack(RETURN, ENV, BITMAP, *params, TEXT, FONT))
            u.reg_write(UC_X86_REG_ESP, STACK)
            u.reg_write(UC_X86_REG_FPCW, 0x143f)
            u.emu_start(0x80c951, RETURN, timeout=30_000_000, count=30_000_000)
            assert u.reg_read(UC_X86_REG_EIP) == RETURN, 'initializer did not return'
            assert u.reg_read(UC_X86_REG_EAX) == BITMAP
            output.extend(pack(side, font_id, *params, len(text)))
            output.extend(text)
            output.extend(pack(api.line_height))
            output.extend(u.mem_read(ENV + 0x14f0 + 4096, 4096))
            output.extend(u.mem_read(PIXELS, side * side * 8))
        finally:
            api.stop(u)
    path = ROOT / '.reference/beta-gdi-font.bin'
    path.write_bytes(output)
    print(len(output), hashlib.sha256(output).hexdigest())

if __name__ == '__main__':
    main()
