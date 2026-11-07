# WestPlay: a phone in a person's right hand, for the phone's animations ("celular", "celular_andando"), joined
# into the body on the hand's bone (docs/personagem/INCLUIR_PERSONAGEM.md, "Celular").
#
#   blender -b <x_animada.blend> --python celular_na_mao.py -- <phone.fbx> <out_celular.blend> <tex.png> \
#       [frame 60] [along 0.075] [up 0.031] [side 0.004] [tilt 0]
#
# The phone: 0.147 m long, two colours (a dark screen, a graphite body: no brand's icons), lying on the palm with
# its screen to the eyes, placed at a frame of "celular" (the texting pose): `along` metres from the wrist toward
# the fingers, `up` off the hand's bones toward the palm's side (under 0.03 the palm shows through the screen:
# owner 2026-11-06), `side` toward the thumb, `tilt` degrees. Renders three looks (<out>_olho/_lado/_frente.png)
# to check it; then, with it right, moved to the hand's rest pose and joined, every vertex on the hand's bone.
import bpy, math, os, sys
from mathutils import Vector, Matrix

A = sys.argv[sys.argv.index('--') + 1:]
PHONE, OUT, TEX = A[0], A[1], A[2]
FRAME = int(A[3]) if len(A) > 3 else 60
ALONG, UP, SIDE, TILT = [float(x) for x in (A[4:8] + ["0.075", "0.031", "0.004", "0"][len(A[4:8]):])]
LENGTH = 0.147

arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
body = [o for o in bpy.data.objects if o.type == 'MESH' and any(m.type == 'ARMATURE' for m in o.modifiers)][0]
sc = bpy.context.scene
bone = lambda n: [b for b in arm.pose.bones if b.name.endswith(n)][0]
arm.animation_data.action = bpy.data.actions["celular"]; sc.frame_set(FRAME); bpy.context.view_layer.update()
mw = arm.matrix_world
hand_bone = bone("RightHand")
hand = mw @ hand_bone.head; index = mw @ bone("RightHandIndex1").head; thumb = mw @ bone("RightHandThumb1").head
head = mw @ bone("Head").head; top = mw @ bone("HeadTop_End").head; eye = head + (top - head) * 0.45
f = (index - hand).normalized()                     # toward the fingers
p = (thumb - hand).cross(f).normalized()            # off the palm
if p.dot(eye - hand) < 0: p = -p
s = f.cross(p).normalized()

before = set(bpy.data.objects)
bpy.ops.import_scene.fbx(filepath=PHONE)
ph = [o for o in bpy.data.objects if o not in before and o.type == 'MESH'][0]; ph.name = "celular"
for o in bpy.data.objects: o.select_set(o == ph)
bpy.context.view_layer.objects.active = ph
bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
me = ph.data; zs = [v.co.z for v in me.vertices]; k = LENGTH / (max(zs) - min(zs))
c = sum((v.co for v in me.vertices), Vector()) / len(me.vertices)
for v in me.vertices: v.co = (v.co - c) * k
# two colours: the screen (the faces looking -Y) and the rest
img = bpy.data.images.new(os.path.splitext(os.path.basename(TEX))[0], 64, 32); px = []
for y in range(32):
    for x in range(64):
        px += [0.035, 0.035, 0.04, 1] if x < 32 else [0.05 + 0.06 * y / 32, 0.08 + 0.06 * y / 32, 0.13 + 0.08 * y / 32, 1]
img.pixels = px
uv = me.uv_layers.active.data
for poly in me.polygons:
    for li in poly.loop_indices: uv[li].uv = (0.75, 0.5) if poly.normal.y < -0.8 else (0.25, 0.5)
mat = bpy.data.materials.new("celular"); mat.use_nodes = True
t = mat.node_tree.nodes.new('ShaderNodeTexImage'); t.image = img; t.interpolation = 'Closest'
mat.node_tree.links.new(t.outputs['Color'], mat.node_tree.nodes["Principled BSDF"].inputs['Base Color'])
me.materials.clear(); me.materials.append(mat)
# the phone's width along s, its screen (-Y) toward the eyes, its length along the fingers
R = Matrix(((s.x, -p.x, f.x), (s.y, -p.y, f.y), (s.z, -p.z, f.z))).to_4x4()
ph.matrix_world = Matrix.Translation(hand + f * ALONG + p * UP + s * SIDE) @ R @ Matrix.Rotation(math.radians(TILT), 4, 'X')
bpy.context.view_layer.update()

# the looks
for o in list(bpy.data.objects):
    if o.type in ('CAMERA', 'LIGHT'): bpy.data.objects.remove(o)
sc.render.engine = 'BLENDER_EEVEE'; w = bpy.data.worlds.new("w"); sc.world = w; w.color = (0.35, 0.35, 0.37)
sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", 'SUN')); sun.data.energy = 3
sun.rotation_euler = (math.radians(40), 0, math.radians(-30)); sc.collection.objects.link(sun)
cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam")); sc.collection.objects.link(cam); sc.camera = cam
cam.data.clip_start = 0.01
sc.render.resolution_x = 450; sc.render.resolution_y = 450
target = hand + f * 0.06
for name, pos, lens in [("olho", eye + (target - eye).normalized() * 0.15, 35),
                        ("lado", target + Vector((-0.35, 0, 0.05)), 50), ("frente", target + Vector((0.05, -0.35, 0.05)), 50)]:
    cam.location = pos; cam.rotation_euler = (target - pos).to_track_quat('-Z', 'Y').to_euler(); cam.data.lens = lens
    sc.render.filepath = os.path.splitext(OUT)[0] + "_" + name + ".png"; bpy.ops.render.render(write_still=True)
for o in (cam, sun): bpy.data.objects.remove(o)

# into the body: where the hand is at rest, then joined on the hand's bone
pose = mw @ hand_bone.matrix
rest = mw @ arm.data.bones[hand_bone.name].matrix_local
to_rest = rest @ pose.inverted()
for v in ph.data.vertices: v.co = to_rest @ (ph.matrix_world @ v.co)
ph.matrix_world = Matrix.Identity(4)
n = len(body.data.vertices)
for o in bpy.data.objects: o.select_set(o in (body, ph))
bpy.context.view_layer.objects.active = body
bpy.ops.object.join()
body.vertex_groups[hand_bone.name].add(list(range(n, len(body.data.vertices))), 1.0, 'REPLACE')
img.filepath_raw = TEX; img.file_format = 'PNG'; img.save()
bpy.ops.file.pack_all()
arm.animation_data.action = bpy.data.actions["andar"]; sc.frame_set(1)
bpy.ops.wm.save_as_mainfile(filepath=OUT)
print("PHONE joined:", len(body.data.vertices) - n, "vertices on", hand_bone.name, "->", OUT)
