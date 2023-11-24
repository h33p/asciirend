"""
asciirend-py thin wrapper

This wrapper is an example of simple usage of asciirend. The intention is to allow loading json
scene descriptions and render them. No futher functionality is planned. This is great to perform
server-side rendering of 3D graphics, but not extensive enough for interactivity.
"""

from wasmtime.loader import store
import asciirend.wasm as ar
from asciirend.wasm import new_frame, update_camera, new_scene, remove_scene, event_focus, event_mouse_pos, event_mouse_pos_clear, event_mouse_button_state, event_scroll, set_dither_count_frames
import struct

class String:
    def __init__(self, *args):
        if len(args) == 2:
            self.ptr = args[0]
            self.length = args[1]
        elif len(args) == 1:
            text_bytes = args[0].encode('utf-8')
            self.length = len(text_bytes)
            self.ptr = ar.alloc_string(self.length)
            ar.memory.write(store, text_bytes, self.ptr)

    def free(self):
        if self.ptr is not None:
            ar.dealloc_string(self.ptr, self.length)
            self.ptr = None

    def read(self):
        b = ar.memory.read(store, self.ptr, self.ptr + self.length)
        return b.decode("utf-8")

class StrTup:
    def __init__(self, ptr):
        self.ptr = ptr

    def free(self):
        if self.ptr is not None:
            ar.dealloc_strtup(self.ptr)
            self.ptr = None

    def to_string(self):
        data = ar.memory.read(store, self.ptr, self.ptr + 8)
        ptr, length = struct.unpack('<II', data)
        self.free()
        return String(ptr, length)

class Pixel:
    def __init__(self, vals):
        self.r = vals[0]
        self.g = vals[1]
        self.b = vals[2]
        self.c = vals[3]

class Pixels:
    def __init__(self, ptr, w, h):
        self.ptr = ptr
        self.w = w
        self.h = h

    def free(self):
        ar.free_raw_pixels(self.ptr, self.w, self.h)

    def read(self):
        out = []

        data = ar.memory.read(store, self.ptr, self.ptr + (self.w * self.h) * 4)

        for y in range(self.h):
            row = []
            for x in range(self.w):
                pos = (self.w * y + x) * 4
                d = data[pos:pos + 4]
                row.append(Pixel(d))
            out.append(row)

        return out

    def to_string(self):
        out = ""
        for row in self.read():
            out += bytes([x.c for x in row]).decode('utf-8')
            out += '\n'
        return out

    def draw(self):
        for row in self.read():
            print(bytes([x.c for x in row]).decode('utf-8'))

def scene_from_json(json):
    s = String(json)
    scene = ar.scene_from_json(s.ptr, s.length)
    s.free()
    return scene

def scene_to_json(scene):
    st = ar.scene_to_json_tuple(scene)
    st = StrTup(st)
    st = st.to_string()
    json = st.read()
    st.free()
    return json

def render(scene, colors, w, h):
    return Pixels(ar.render_raw(scene, colors, w, h), w, h)

def ascii_render_pixels(scene_desc, colors, w, h, aspect, is_ortho, fov, znear, zfar):
    scene = scene_from_json(scene_desc)
    assert scene != -1;
    if is_ortho:
        proj = 1
    else:
        proj = 0
    new_frame(scene, proj, w, h, aspect, fov, znear, zfar)
    ret = render(scene, colors, w, h)
    remove_scene(scene)
    return ret

# This one mimicks the javascript API, although here we are able to set arbitrary dimensions
#
# If you wish to receive [Pixel], instead of string, use ascii_render_pixels, but be sure to free them!
def ascii_render(scene_desc, colors, w, h, aspect, is_ortho, fov, znear, zfar):
    pixels = ascii_render_pixels(scene_desc, colors, w, h, aspect, is_ortho, fov, znear, zfar)
    ret = pixels.to_string()
    pixels.free()
    return ret
