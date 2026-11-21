# WestPlay: refuelling by hand (owner 2026-11-12: "a real arm, the character's own, holding the pump's nozzle").
# Makes the "abastecer" animation (game name `fuel`) of a person from their own `parado`: standing, the right arm
# reaching forward and down to the truck's tank, the hand closed round the nozzle's grip; held still. Saves it in
# the scene (as `engatar.py` does), then joins a pump's nozzle into the body on the right hand's bone, as the phone
# is (`celular_na_mao.py`), and saves that as another scene for the `xx_nzbody` model (headless, first person).
#
#   blender -b <x_animada.blend> --python abastecer.py -- <out_bico.blend> <nozzle_tex.png> [<preview folder>]
#       [reach 0.78] [drop 0.42] [inward 0.35] [curl 75]
#
# No animation of Mixamo's is added: the body is `parado`'s first frame, the arm placed by reaching for a point in
# front of the right shoulder (IK) and written into the action, the fingers curled. The nozzle is plain shapes
# (a body, a spout bending down, a guard and lever, a grip, a length of hose), three colours on a small texture,
# no brand's marks.
import bpy, bmesh, math, os, sys
from mathutils import Vector, Matrix, Quaternion

A = sys.argv[sys.argv.index("--") + 1:]
OUT, TEX = A[0], A[1]
PREVIEW = A[2] if len(A) > 2 and A[2] else None
REACH, DROP, INWARD, CURL = [float(x) for x in (A[3:7] + ["0.78", "0.42", "0.35", "75"][len(A[3:7]):])]
SOURCE, NAME, FRAMES = "parado", "abastecer", 31
# The grip in the fist: this much of the way to the knuckles ahead, in toward the palm, down from the hand's bone.
GRIP_AHEAD, GRIP_IN, GRIP_DOWN = 0.62, 0.03, 0.035

arm = bpy.data.objects["esqueleto"]
body = [o for o in bpy.data.objects if o.type == "MESH" and any(m.type == "ARMATURE" for m in o.modifiers)][0]
scene = bpy.context.scene
world = arm.matrix_world
bone = lambda name: [b for b in arm.pose.bones if b.name.endswith(name)][0]

# The body: parado's first frame, held.
source = bpy.data.actions[SOURCE]
if NAME in bpy.data.actions:
    bpy.data.actions.remove(bpy.data.actions[NAME])
action = bpy.data.actions.new(NAME)
action.use_fake_user = True
start = source.frame_range[0]
for curve in source.fcurves:
    made = action.fcurves.new(curve.data_path, index=curve.array_index, action_group=curve.group.name if curve.group else "")
    value = curve.evaluate(start)
    for f in (1, FRAMES):
        made.keyframe_points.insert(f, value).interpolation = "LINEAR"
arm.animation_data_create()
arm.animation_data.action = action
scene.frame_set(1)
bpy.context.view_layer.update()

# The right arm reaching out: in front of the shoulder, down, a little toward the middle (the person faces -Y).
upper, fore, hand = bone("RightArm"), bone("RightForeArm"), bone("RightHand")
shoulder = world @ upper.head
reach = (upper.length + fore.length) * world.to_scale().x
middle = world @ bone("Spine2").head
target = Vector((shoulder.x + (middle.x - shoulder.x) * INWARD, shoulder.y - reach * REACH, shoulder.z - reach * DROP))
empty = bpy.data.objects.new("alvo_bico", None)
scene.collection.objects.link(empty)
empty.location = target
pole = bpy.data.objects.new("polo_bico", None)
scene.collection.objects.link(pole)
# The elbow out to the side and down, as one holds a nozzle.
pole.location = shoulder + Vector(((shoulder.x - middle.x) * 2.5, 0.1, -reach * 0.6))
ik = fore.constraints.new("IK")
ik.target, ik.pole_target, ik.chain_count = empty, pole, 2
ik.pole_angle = math.radians(-90)
bpy.context.view_layer.update()
turns = {}
for pose in (upper, fore):
    local = arm.convert_space(pose_bone=pose, matrix=pose.matrix, from_space="POSE", to_space="LOCAL")
    turns[pose.name] = local.to_quaternion()
fore.constraints.remove(ik)
bpy.data.objects.remove(empty)
bpy.data.objects.remove(pole)
print("REACH target", tuple(round(v, 3) for v in target), "reach", round(reach, 3))

def write(name, q):
    path = 'pose.bones["%s"].rotation_quaternion' % name
    for i in range(4):
        curve = action.fcurves.find(path, index=i) or action.fcurves.new(path, index=i, action_group=name)
        curve.keyframe_points.clear()
        for f in (1, FRAMES):
            curve.keyframe_points.insert(f, q[i]).interpolation = "LINEAR"

for name, q in turns.items():
    write(name, q)
bpy.context.view_layer.update()
# The hand, worked out in the armature's space from its bones at rest (the T-pose: fingers out, thumb ahead, palm
# down): turned so its fingers point ahead and a little down (`DOWNWARD`), the thumb up, the palm to the middle.
DOWNWARD = 0.3
to_arm = world.inverted().to_3x3()
rest_of = lambda name: arm.data.bones[bone(name).name].matrix_local
hand_rest = rest_of("RightHand")
fingers_rest = (rest_of("RightHandIndex1").translation - hand_rest.translation).normalized()
thumb_rest = rest_of("RightHandThumb1").translation - hand_rest.translation
thumb_rest = (thumb_rest - fingers_rest * thumb_rest.dot(fingers_rest)).normalized()
palm_rest = thumb_rest.cross(fingers_rest).normalized()
ahead = (to_arm @ Vector((0.0, -1.0, -DOWNWARD))).normalized()
up_arm = to_arm @ Vector((0.0, 0.0, 1.0))
thumb_to = (up_arm - ahead * up_arm.dot(ahead)).normalized()
palm_to = thumb_to.cross(ahead).normalized()
rest_frame = Matrix((fingers_rest, thumb_rest, palm_rest)).transposed()
to_frame = Matrix((ahead, thumb_to, palm_to)).transposed()
turn = to_frame @ rest_frame.inverted()
posed = Matrix.Translation(hand.matrix.translation) @ (turn @ hand_rest.to_3x3()).to_4x4()
hand.matrix = posed
bpy.context.view_layer.update()
write(hand.name, arm.convert_space(pose_bone=hand, matrix=hand.matrix, from_space="POSE", to_space="LOCAL").to_quaternion())
# The fingers closed round the grip: each joint bent toward the palm (about the axis across the hand), the thumb
# a little in.
curl_axis = fingers_rest.cross(palm_rest).normalized()
thumb_axis = thumb_rest.cross(palm_rest).normalized()
for b in arm.pose.bones:
    n = b.name
    if not n.startswith("mixamorig:RightHand") or n[-1] not in "123":
        continue
    axis, angle = (thumb_axis, 25.0) if "Thumb" in n else (curl_axis, CURL)
    if "Thumb" in n and n[-1] == "1":
        continue
    local = (arm.data.bones[n].matrix_local.to_3x3().inverted() @ axis).normalized()
    write(n, Quaternion(local, math.radians(angle)))
scene.frame_set(1)
bpy.context.view_layer.update()
print("ACTION", NAME, tuple(action.frame_range), len(action.fcurves))
bpy.ops.wm.save_mainfile()

# The nozzle, in the closed hand: its barrel ahead (the way the hand points), the grip upright through the fist,
# the thumb's side up.
h = world @ hand.head
ahead_w = (world.to_3x3() @ ahead).normalized()
up_w = (world.to_3x3() @ thumb_to).normalized()
palm_w = (world.to_3x3() @ palm_to).normalized()

def smooth(points, per=6):
    # A Catmull-Rom curve through the points: bends without corners.
    pts = [points[0]] + list(points) + [points[-1]]
    out = []
    for i in range(1, len(pts) - 2):
        p0, p1, p2, p3 = pts[i - 1], pts[i], pts[i + 1], pts[i + 2]
        for k in range(per):
            t = k / per
            out.append(0.5 * ((2 * p1) + (-p0 + p2) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t * t + (-p0 + 3 * p1 - 3 * p2 + p3) * t * t * t))
    out.append(points[-1])
    return out

def tube(bm, points, radius, sides=16, rounded=True):
    # Rings along the path; each end closed by a small dome (no flat lid with a sharp rim).
    rings = []
    frames = []
    u = None
    for i, at in enumerate(points):
        along = (points[min(i + 1, len(points) - 1)] - points[max(i - 1, 0)]).normalized()
        if u is None:
            other = Vector((0, 0, 1)) if abs(along.z) < 0.9 else Vector((1, 0, 0))
            u = along.cross(other).normalized()
        else:
            # Carried along the bend (no ring turned against the last: the guard was kinked, 2026-11-14).
            u = (u - along * u.dot(along)).normalized()
        v = along.cross(u)
        frames.append((along, u, v))
        rings.append([bm.verts.new(at + (u * math.cos(k / sides * math.tau) + v * math.sin(k / sides * math.tau)) * radius) for k in range(sides)])
    for a, b in zip(rings, rings[1:]):
        for k in range(sides):
            bm.faces.new((a[k], a[(k + 1) % sides], b[(k + 1) % sides], b[k]))
    for end, sign in ((0, -1), (len(points) - 1, 1)):
        along, u, v = frames[end]
        ring = rings[end]
        if rounded:
            # Two smaller rings out past the end, then a point: a shallow dome.
            prev = ring
            for f in (0.75, 0.4):
                lift = math.sqrt(1 - f * f) * radius * 0.45
                new = [bm.verts.new(points[end] + along * sign * lift + (u * math.cos(k / sides * math.tau) + v * math.sin(k / sides * math.tau)) * radius * f) for k in range(sides)]
                for k in range(sides):
                    quad = (prev[k], prev[(k + 1) % sides], new[(k + 1) % sides], new[k])
                    bm.faces.new(quad if sign > 0 else quad[::-1])
                prev = new
            tip = bm.verts.new(points[end] + along * sign * radius * 0.45)
            for k in range(sides):
                tri = (prev[k], prev[(k + 1) % sides], tip)
                bm.faces.new(tri if sign > 0 else tri[::-1])
        else:
            bm.faces.new(ring if sign > 0 else ring[::-1])

def rounded_box(bm, low, high, radius, segments=3):
    # A box with its edges and corners rounded off (bevelled).
    made = bmesh.ops.create_cube(bm, size=1.0)["verts"]
    for v in made:
        v.co = Vector(tuple((high[i] + low[i]) / 2 + v.co[i] * (high[i] - low[i]) for i in range(3)))
    edges = list({e for v in made for e in v.link_edges})
    bmesh.ops.bevel(bm, geom=edges + made, offset=radius, offset_type="OFFSET", segments=segments, profile=0.5,
                    affect="EDGES", clamp_overlap=True)

# In the nozzle's own space: x to the right, y up (the thumb's way), -z ahead (the barrel), the grip at the origin.
combined = bmesh.new()
# The body's own texture coordinates' layer by name: joined, the nozzle's are kept in it (another name and they were
# lost, the nozzle all red: owner 2026-11-12).
uv_layer = combined.loops.layers.uv.new(body.data.uv_layers.active.name if body.data.uv_layers.active else "UVMap")
uv_of = {"body": (0.25, 0.25), "grip": (0.25, 0.75), "metal": (0.75, 0.25), "hose": (0.75, 0.75)}
def coloured(colour, make):
    # Each shape's faces given its colour's place on the texture as they're made.
    before = set(combined.faces)
    make(combined)
    for face in combined.faces:
        if face not in before:
            for loop in face.loops:
                loop[uv_layer].uv = uv_of[colour]
# The body over the fist, its corners rounded (owner 2026-11-14: "more rounded, prettier").
coloured("body", lambda bm: rounded_box(bm, (-0.021, 0.034, -0.168), (0.021, 0.086, 0.032), 0.014))
# The grip through the fist: rounder still, the dark rubber a hand holds.
coloured("grip", lambda bm: rounded_box(bm, (-0.0175, -0.072, -0.031), (0.0175, 0.042, 0.026), 0.0135, 4))
# The guard: one bent bar from the grip's foot round under the lever to the body's front (it was two boxes).
coloured("hose", lambda bm: tube(bm, smooth([Vector((0, -0.058, -0.028)), Vector((0, -0.066, -0.07)), Vector((0, -0.058, -0.125)),
                                             Vector((0, -0.03, -0.157)), Vector((0, 0.04, -0.158))], 5), 0.0055, 10))
# The lever under the body, inside the guard, its edges rounded.
coloured("metal", lambda bm: rounded_box(bm, (-0.006, -0.022, -0.128), (0.006, 0.034, -0.047), 0.0045))
# The spout: a collar where it leaves the body, then a smooth bend down to its tip, a ring near the end.
coloured("metal", lambda bm: tube(bm, [Vector((0, 0.06, -0.162)), Vector((0, 0.06, -0.192))], 0.017, 18))
coloured("metal", lambda bm: tube(bm, smooth([Vector((0, 0.06, -0.19)), Vector((0, 0.06, -0.245)), Vector((0, 0.045, -0.293)),
                                              Vector((0, 0.015, -0.33)), Vector((0, -0.012, -0.35))], 6), 0.0105, 16))
coloured("metal", lambda bm: tube(bm, [Vector((0, 0.06, -0.226)), Vector((0, 0.06, -0.236))], 0.013, 16))
# The hose out of the grip's foot, a metal sleeve at it, then straight down under the arm, bending smoothly
# (owner 2026-11-12: through the forearm, it showed).
coloured("metal", lambda bm: tube(bm, [Vector((0, -0.064, 0.011)), Vector((0, -0.1, 0.014))], 0.0215, 16))
coloured("hose", lambda bm: tube(bm, smooth([Vector((0, -0.095, 0.014)), Vector((0, -0.14, 0.02)), Vector((0, -0.4, 0.05)),
                                             Vector((0, -0.75, 0.09)), Vector((0, -1.0, 0.18)), Vector((0, -1.12, 0.45))], 5), 0.019, 14))
mesh = bpy.data.meshes.new("bico")
bmesh.ops.recalc_face_normals(combined, faces=combined.faces)
combined.to_mesh(mesh); combined.free()
# Smooth shading: the rounded parts read round (owner 2026-11-14).
for poly in mesh.polygons: poly.use_smooth = True
nozzle = bpy.data.objects.new("bico", mesh)
scene.collection.objects.link(nozzle)
# Its texture: the body red (owner 2026-11-12: "red, to stand out"), the grip dark rubber (2026-11-14), the spout and lever metal, the hose
# black and a little shiny.
img = bpy.data.images.new(os.path.splitext(os.path.basename(TEX))[0], 32, 32)
px = []
for y in range(32):
    for x in range(32):
        px += ([0.72, 0.04, 0.03, 1] if y < 16 else [0.06, 0.06, 0.065, 1]) if x < 16 else ([0.62, 0.63, 0.65, 1] if y < 16 else [0.02, 0.02, 0.02, 1])
img.pixels = px
mat = bpy.data.materials.new("bico"); mat.use_nodes = True
t = mat.node_tree.nodes.new("ShaderNodeTexImage"); t.image = img; t.interpolation = "Closest"
mat.node_tree.links.new(t.outputs["Color"], mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"])
mat["westplay_tex"] = "nozzle_tex"
mesh.materials.append(mat)
# Into the hand: its x along the hand's side, y the thumb's way, -z the knuckles' (the barrel ahead).
side_w = up_w.cross(-ahead_w).normalized()
R = Matrix(((side_w.x, up_w.x, -ahead_w.x), (side_w.y, up_w.y, -ahead_w.y), (side_w.z, up_w.z, -ahead_w.z))).to_4x4()
knuckles = (world @ bone("RightHandIndex1").head - h).length
grip = h + ahead_w * knuckles * GRIP_AHEAD + palm_w * GRIP_IN - up_w * GRIP_DOWN
nozzle.matrix_world = Matrix.Translation(grip) @ R
print("GRIP", tuple(round(v, 3) for v in grip), "knuckles", round(knuckles, 3))
bpy.context.view_layer.update()

if PREVIEW:
    os.makedirs(PREVIEW, exist_ok=True)
    for o in list(bpy.data.objects):
        if o.type in ("CAMERA", "LIGHT"): bpy.data.objects.remove(o)
    scene.render.engine = "BLENDER_EEVEE"; w = bpy.data.worlds.new("w"); scene.world = w; w.color = (0.35, 0.35, 0.37)
    sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN")); sun.data.energy = 3
    sun.rotation_euler = (math.radians(40), 0, math.radians(-30)); scene.collection.objects.link(sun)
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam")); scene.collection.objects.link(cam); scene.camera = cam
    cam.data.clip_start = 0.01
    scene.render.resolution_x = scene.render.resolution_y = 500
    head = world @ bone("Head").head; top = world @ bone("HeadTop_End").head; eye = head + (top - head) * 0.45
    stem = os.path.splitext(os.path.basename(OUT))[0]
    for name, pos, look, lens in [("olho", eye + Vector((0, -0.12, 0.0)), grip, 22),
                                  ("lado", grip + Vector((-1.2, 0.1, 0.2)), grip + Vector((0, 0.15, 0.2)), 35),
                                  ("frente", grip + Vector((0.6, -1.6, 0.3)), grip + Vector((0, 0.2, 0.2)), 35),
                                  ("mao", grip + Vector((-0.25, -0.25, 0.15)), grip, 50)]:
        cam.location = pos; cam.rotation_euler = (look - pos).to_track_quat("-Z", "Y").to_euler(); cam.data.lens = lens
        scene.render.filepath = os.path.join(PREVIEW, stem + "_" + name + ".png"); bpy.ops.render.render(write_still=True)
    for o in (cam, sun): bpy.data.objects.remove(o)

# Into the body: where the hand is at rest, then joined on the hand's bone.
pose = world @ hand.matrix
rest = world @ arm.data.bones[hand.name].matrix_local
to_rest = rest @ pose.inverted()
for v in nozzle.data.vertices: v.co = to_rest @ (nozzle.matrix_world @ v.co)
nozzle.matrix_world = Matrix.Identity(4)
n = len(body.data.vertices)
for o in bpy.data.objects: o.select_set(o in (body, nozzle))
bpy.context.view_layer.objects.active = body
bpy.ops.object.join()
body.vertex_groups[hand.name].add(list(range(n, len(body.data.vertices))), 1.0, "REPLACE")
img.filepath_raw = TEX; img.file_format = "PNG"; img.save()
bpy.ops.file.pack_all()
arm.animation_data.action = action; scene.frame_set(1)
bpy.ops.wm.save_as_mainfile(filepath=OUT)
print("NOZZLE joined:", len(body.data.vertices) - n, "vertices on", hand.name, "->", OUT)
