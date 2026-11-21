# WestPlay: makes the "engatar" animation (game name `work`) of a person, from their own `agachar`: down on a knee,
# both hands working at something in front of them (the right one turning, as at a crank; the left one holding),
# and up again. Played once, from start to end (owner 2026-11-11: looped in the middle, it was seen crouching three
# times): down by frame 20, the hands at work to frame 80, up by 100. For coupling and uncoupling the trailer by hand (owner 2026-11-11: "an animation with the arms,
# moving them, instead of the arms still").
#
#   blender -b <x_animada.blend> --python engatar.py -- [<preview folder>]
#
# No animation of Mixamo's is added: the legs and the body are the person's own `agachar` (down to its frame 20,
# held, up from it), and the arms are placed by reaching for two points in front of the chest (IK), written into
# the action frame by frame. The scene is saved with the new action; a preview folder gets three pictures of it.
import bpy, math, os, sys
from mathutils import Vector

ARGS = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
SOURCE, NAME = "agachar", "engatar"
DOWN = 20            # agachar's frame on the knee
WORK = 60            # frames working (at 30 a second)
EASE = 6             # frames the arms take to reach out, and to come back
TURN = 30            # frames a turn of the right hand takes

arm = bpy.data.objects["esqueleto"]
scene = bpy.context.scene
source = bpy.data.actions[SOURCE]
last = int(source.frame_range[1])
total = DOWN + WORK + (last - DOWN)
print("ENGATAR", SOURCE, "frames", last, "->", total)

# The body: agachar down to the knee, held, and up again.
if NAME in bpy.data.actions:
    bpy.data.actions.remove(bpy.data.actions[NAME])
action = bpy.data.actions.new(NAME)
action.use_fake_user = True
def from_frame(f):
    return f if f <= DOWN else DOWN if f <= DOWN + WORK else f - WORK
for curve in source.fcurves:
    made = action.fcurves.new(curve.data_path, index=curve.array_index, action_group=curve.group.name if curve.group else "")
    made.keyframe_points.add(total)
    for i, point in enumerate(made.keyframe_points):
        f = i + 1
        point.co = (f, curve.evaluate(from_frame(f)))
        point.interpolation = "LINEAR"
    made.update()
arm.animation_data_create()
arm.animation_data.action = action

def bone(side, part):
    return arm.pose.bones["mixamorig:%s%s" % (side, part)]

# Where the hands work: in front of the chest, from where the shoulders are on the knee.
scene.frame_set(DOWN)
bpy.context.view_layer.update()
world = arm.matrix_world
targets = {}
for side, sign in (("Right", -1.0), ("Left", 1.0)):
    upper, fore, hand = bone(side, "Arm"), bone(side, "ForeArm"), bone(side, "Hand")
    shoulder = world @ upper.head
    reach = (upper.length + fore.length) * world.to_scale().x
    # The person faces -Y, bent over their knee: the hands well out in front and low (at something by the ground,
    # under the face: nearer and higher, they were at the chin), close together.
    centre = Vector((shoulder.x * 0.6, shoulder.y - reach * 0.8, shoulder.z - reach * (0.56 if side == "Right" else 0.68)))
    empty = bpy.data.objects.new("alvo_" + side, None)
    scene.collection.objects.link(empty)
    empty.location = centre
    ik = fore.constraints.new("IK")
    ik.target = empty
    ik.chain_count = 2
    ik.influence = 0.0
    targets[side] = (empty, centre, reach, ik, hand)
    print("TARGET", side, tuple(round(v, 2) for v in centre), "reach", round(reach, 2), "shoulder", tuple(round(v, 2) for v in shoulder))

def eased(x):
    x = max(0.0, min(1.0, x))
    return x * x * (3 - 2 * x)

# The arms, frame by frame: reaching out as the knee comes down, working, back as the person gets up.
first, final = DOWN - EASE, DOWN + WORK + EASE
keys = {}
for f in range(first, final + 1):
    phase = (f - DOWN) / TURN * math.tau
    for side, (empty, centre, reach, ik, hand) in targets.items():
        ik.influence = eased(min((f - first) / EASE, (final - f) / EASE))
        if side == "Right":
            # Turning: a circle facing the person, a little pull with it.
            r = reach * 0.14
            empty.location = centre + Vector((r * math.cos(phase), reach * 0.03 * math.sin(phase * 2), r * math.sin(phase)))
        else:
            # Holding: steady, giving a little with each turn.
            empty.location = centre + Vector((0.0, reach * 0.04 * math.sin(phase), reach * 0.05 * math.cos(phase)))
    scene.frame_set(f)
    bpy.context.view_layer.update()
    for side in targets:
        for part in ("Arm", "ForeArm"):
            pose = bone(side, part)
            local = arm.convert_space(pose_bone=pose, matrix=pose.matrix, from_space="POSE", to_space="LOCAL")
            keys.setdefault(pose.name, {})[f] = local.to_quaternion()

for side, (empty, centre, reach, ik, hand) in targets.items():
    bone(side, "ForeArm").constraints.remove(ik)
    bpy.data.objects.remove(empty)

# Written into the action (each bone's turns kept on the same side of the quaternion, frame to frame).
for name, frames in keys.items():
    path = 'pose.bones["%s"].rotation_quaternion' % name
    curves = [action.fcurves.find(path, index=i) for i in range(4)]
    before = None
    for f in sorted(frames):
        q = frames[f]
        if before is None:
            before = Vector([c.evaluate(f - 1) for c in curves])
        if Vector(q).dot(before) < 0:
            q = -q
        before = Vector(q)
        for i, curve in enumerate(curves):
            curve.keyframe_points[f - 1].co = (f, q[i])
    for curve in curves:
        curve.update()

# The right hand turns at the wrist with the crank, a little.
wrist = 'pose.bones["mixamorig:RightHand"].rotation_quaternion'
curves = [action.fcurves.find(wrist, index=i) for i in range(4)]
if all(curves):
    from mathutils import Quaternion
    for f in range(first, final + 1):
        amount = eased(min((f - first) / EASE, (final - f) / EASE)) * 0.35 * math.sin((f - DOWN) / TURN * math.tau)
        base = Quaternion([c.evaluate(f) for c in curves])
        q = base @ Quaternion((0.0, 1.0, 0.0), amount)
        for i, curve in enumerate(curves):
            curve.keyframe_points[f - 1].co = (f, q[i])
    for curve in curves:
        curve.update()

scene.frame_set(1)
bpy.context.view_layer.update()
print("ACTION", NAME, tuple(action.frame_range), len(action.fcurves))
bpy.ops.wm.save_mainfile()

# Pictures of it, to look at before exporting: from the person's front-left, at three moments of the work.
if ARGS:
    out = ARGS[0]
    os.makedirs(out, exist_ok=True)
    camera = bpy.data.objects.new("previa_camera", bpy.data.cameras.new("previa_camera"))
    scene.collection.objects.link(camera)
    camera.location = (1.9, -2.3, 1.0)
    look = Vector((0.0, -0.2, 0.55)) - camera.location
    camera.rotation_euler = look.to_track_quat("-Z", "Y").to_euler()
    scene.camera = camera
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.display.shading.light = "STUDIO"
    scene.display.shading.color_type = "TEXTURE"
    scene.render.resolution_x, scene.render.resolution_y = 560, 560
    scene.render.film_transparent = False
    stem = os.path.splitext(os.path.basename(bpy.data.filepath))[0]
    for f in (DOWN, DOWN + TURN // 2, DOWN + TURN, DOWN + WORK - 6, total - 8):
        scene.frame_set(f)
        scene.render.filepath = os.path.join(out, "%s_%03d.png" % (stem, f))
        bpy.ops.render.render(write_still=True)
        print("PREVIEW", scene.render.filepath)
