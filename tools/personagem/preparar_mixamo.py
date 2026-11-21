# WestPlay: a new person's model made ready for Mixamo (docs/personagem/INCLUIR_PERSONAGEM.md, step 1).
#   blender -b [model.blend] --python preparar_mixamo.py -- <out dir> <name> <height m> [model.fbx] [eye=x,z,rx,rz]
# Takes the one mesh (from the .blend opened, or the FBX given), stands it on the floor facing -Y at <height>
# (its top: a hat included), its transforms applied; its colour texture as <name>_cor.png at 2048x2048; and writes
# <name>.blend, <name>_mixamo.fbx (the texture embedded) and preview_frente.png / preview_rosto.png in <out dir>.
#
# eye=x,z,rx,rz (owner 2026-11-14, person 6's left eye smeared by the AI): the eye at (x, z) on the model as it
# comes (metres; the front view's x and height), rx and rz its half width and height, is painted anew as the other
# eye mirrored. The face's middle is found at its nose; each texel of that eye's area takes the colour the mirrored
# point of the face has, eased into the skin round it.
import bpy, sys, math, os
import numpy as np
from mathutils import Vector
from mathutils.bvhtree import BVHTree

a = sys.argv[sys.argv.index("--") + 1:]
out_dir, name, height = a[0], a[1], float(a[2])
fbx = next((x for x in a[3:] if x.lower().endswith(".fbx")), None)
eye = next((x for x in a[3:] if x.startswith("eye=")), None)
os.makedirs(out_dir, exist_ok=True)

if fbx:
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.fbx(filepath=fbx)
for o in list(bpy.data.objects):
    if o.type != "MESH":
        bpy.data.objects.remove(o)
meshes = [o for o in bpy.data.objects if o.type == "MESH"]
assert len(meshes) == 1, "one mesh expected, found %d" % len(meshes)
body = meshes[0]
body.name = name
body.data.name = name

# The colour texture: the first image the material shows.
image = None
for m in body.data.materials:
    if m and m.use_nodes:
        for n in m.node_tree.nodes:
            if n.type == "TEX_IMAGE" and n.image:
                image = image or n.image
assert image is not None, "no colour texture"
W, H = image.size
print("texture", image.name, W, H)
pixels = np.array(image.pixels[:], dtype=np.float32).reshape(H, W, 4)

if eye:
    ex, ez, rx, rz = (float(v) for v in eye[4:].split(","))
    deps = bpy.context.evaluated_depsgraph_get()
    mw = body.matrix_world
    me = body.data
    uvs = me.uv_layers.active.data
    # The model in the world, for rays from the front.
    verts = [mw @ v.co for v in me.vertices]
    polys = [tuple(p.vertices) for p in me.polygons]
    tree = BVHTree.FromPolygons(verts, polys)
    def hit(x, z):
        loc, normal, index, dist = tree.ray_cast(Vector((x, -5.0, z)), Vector((0, 1, 0)))
        if loc is None:
            return None
        p = me.polygons[index]
        corners = [verts[v] for v in p.vertices]
        loops = [uvs[i].uv for i in p.loop_indices]
        # The point's UV: within the polygon's triangle (fan) that holds it, by barycentric weights.
        for k in range(1, len(corners) - 1):
            a0, a1, a2 = corners[0], corners[k], corners[k + 1]
            w = barycentric(loc, a0, a1, a2)
            if w and min(w) >= -1e-4:
                return loops[0] * w[0] + loops[k] * w[1] + loops[k + 1] * w[2]
        return None
    def barycentric(p, a0, a1, a2):
        v0, v1, v2 = a1 - a0, a2 - a0, p - a0
        d00, d01, d11 = v0.dot(v0), v0.dot(v1), v1.dot(v1)
        d20, d21 = v2.dot(v0), v2.dot(v1)
        den = d00 * d11 - d01 * d01
        if abs(den) < 1e-20:
            return None
        v = (d11 * d20 - d01 * d21) / den
        w = (d00 * d21 - d01 * d20) / den
        return (1 - v - w, v, w)
    # The face's middle: the nose's tip, the frontmost point a little under the eyes.
    nose = min((v for v in verts if ez - 0.07 < v.z < ez - 0.015 and abs(v.x - ex) < 0.08), key=lambda v: v.y)
    mid = nose.x
    print("face middle at x=%.4f (nose tip y=%.4f)" % (mid, nose.y))
    def sample(u, v):
        x, y = u * W - 0.5, v * H - 0.5
        x0, y0 = int(math.floor(x)), int(math.floor(y))
        fx, fy = x - x0, y - y0
        x0, y0 = max(0, min(W - 2, x0)), max(0, min(H - 2, y0))
        c = pixels[y0:y0 + 2, x0:x0 + 2]
        top = c[0, 0] * (1 - fx) + c[0, 1] * fx
        bottom = c[1, 0] * (1 - fx) + c[1, 1] * fx
        return top * (1 - fy) + bottom * fy
    # Each texel of the eye's area, from the triangles of the face's front there: the point of the face it
    # shows, and the colour of the mirrored point.
    def front_y(x, z):
        loc = tree.ray_cast(Vector((x, -5.0, z)), Vector((0, 1, 0)))[0]
        return None if loc is None else loc.y
    painted = pixels.copy()
    done = 0
    for p in me.polygons:
        c = mw @ p.center
        if math.hypot((c.x - ex) / (rx * 1.2), (c.z - ez) / (rz * 1.2)) > 1:
            continue
        seen = front_y(c.x, c.z)
        if seen is None or abs(c.y - seen) > 0.002:
            continue
        # The face only: not a hat's brim out in front of it (person 6's, painted skin at first: owner 2026-11-14).
        if c.y < nose.y - 0.005:
            continue
        corners = [verts[v] for v in p.vertices]
        loops = [uvs[i].uv.copy() for i in p.loop_indices]
        for k in range(1, len(corners) - 1):
            tri3 = (corners[0], corners[k], corners[k + 1])
            tri2 = [Vector((l.x * W, l.y * H)) for l in (loops[0], loops[k], loops[k + 1])]
            x0, x1 = int(min(t.x for t in tri2)), int(max(t.x for t in tri2)) + 1
            y0, y1 = int(min(t.y for t in tri2)), int(max(t.y for t in tri2)) + 1
            (ax, ay), (bx, by), (cx, cy) = tri2[0], tri2[1], tri2[2]
            den = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy)
            if abs(den) < 1e-9:
                continue
            for py in range(max(0, y0), min(H, y1 + 1)):
                for px in range(max(0, x0), min(W, x1 + 1)):
                    qx, qy = px + 0.5, py + 0.5
                    l0 = ((by - cy) * (qx - cx) + (cx - bx) * (qy - cy)) / den
                    l1 = ((cy - ay) * (qx - cx) + (ax - cx) * (qy - cy)) / den
                    l2 = 1 - l0 - l1
                    if min(l0, l1, l2) < -0.02:
                        continue
                    point = tri3[0] * l0 + tri3[1] * l1 + tri3[2] * l2
                    d = math.hypot((point.x - ex) / rx, (point.z - ez) / rz)
                    if d > 1:
                        continue
                    ease = 1.0 if d < 0.6 else 0.5 + 0.5 * math.cos((d - 0.6) / 0.4 * math.pi)
                    there = hit(2 * mid - point.x, point.z)
                    if there is None:
                        continue
                    colour = sample(there.x, there.y)
                    painted[py, px] = pixels[py, px] * (1 - ease) + colour * ease
                    done += 1
    print("eye repainted: %d texels" % done)
    pixels = painted

# The texture at 2048x2048, as the standard has it.
out_image = bpy.data.images.new(name + "_cor", W, H, alpha=False)
out_image.pixels.foreach_set(pixels.ravel())
if W != 2048 or H != 2048:
    out_image.scale(2048, 2048)
out_image.filepath_raw = os.path.join(out_dir, name + "_cor.png")
out_image.file_format = "PNG"
out_image.save()
out_image.filepath = out_image.filepath_raw
for m in body.data.materials:
    if m and m.use_nodes:
        for n in m.node_tree.nodes:
            if n.type == "TEX_IMAGE":
                n.image = out_image

# Standing on the floor, facing -Y, at the height asked, its transforms applied.
bpy.context.view_layer.objects.active = body
body.select_set(True)
bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
zs = [v.co.z for v in body.data.vertices]
xs = [v.co.x for v in body.data.vertices]
ys = [v.co.y for v in body.data.vertices]
k = height / (max(zs) - min(zs))
offset = Vector((-(min(xs) + max(xs)) / 2, -(min(ys) + max(ys)) / 2, -min(zs)))
for v in body.data.vertices:
    v.co = (v.co + offset) * k
body.location = (0, 0, 0)
tris = sum(len(p.vertices) - 2 for p in body.data.polygons)
print("height %.3f m, %d triangles" % (height, tris))

bpy.ops.wm.save_as_mainfile(filepath=os.path.join(out_dir, name + ".blend"))
bpy.ops.export_scene.fbx(
    filepath=os.path.join(out_dir, name + "_mixamo.fbx"),
    use_selection=False,
    object_types={"MESH"},
    path_mode="COPY",
    embed_textures=True,
    axis_forward="-Z",
    axis_up="Y",
)

# Previews: the whole front, and the face.
sc = bpy.context.scene
sc.render.engine = "BLENDER_EEVEE"
sc.view_settings.view_transform = "Standard"
world = bpy.data.worlds.new("w")
sc.world = world
world.color = (0.55, 0.55, 0.57)
key = bpy.data.objects.new("key", bpy.data.lights.new("key", "SUN"))
key.data.energy = 3
key.rotation_euler = (math.radians(60), 0, math.radians(-20))
sc.collection.objects.link(key)
cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
sc.collection.objects.link(cam)
sc.camera = cam
cam.data.type = "ORTHO"
cam.rotation_euler = (math.radians(90), 0, 0)
sc.render.resolution_x, sc.render.resolution_y = 900, 900
width = max(xs) * k - min(xs) * k
for label, z, size in (("frente", height / 2, max(height, width) * 1.08), ("rosto", height * 0.89, height * 0.16)):
    cam.location = (0, -5, z)
    cam.data.ortho_scale = size
    sc.render.filepath = os.path.join(out_dir, "preview_" + label + ".png")
    bpy.ops.render.render(write_still=True)
print("DONE", out_dir)
