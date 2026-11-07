# WestPlay: the crouch, smooth (owner 2026-11-06: "the crouch's physics, smooth, nothing strange"), from Mixamo's
# "Standing To Crouched" and "Crouched Walking" (already in the scene as agachar_descer and andar_agachado):
#
# - agachar: down to one knee (Standing To Crouched) and back up (the same, played backwards: in place, the same
#   poses; Mixamo's "Crouch To Stand" steps forward off the capsule and stands a second more), its first and last
#   frames standing's first (6 frames each way blended into it). 40 frames: down by 20, up by 40.
# - agachado: the knee pose, held (what crouching still is; the way from walking crouched ends in it).
# - andar_agachado: centred over the feet (it came 13 cm to one side), the lowest foot on the floor.
# - agachado_andar: from the knee into walking crouched, the legs already stepping (N frames, then the walk goes on
#   from its frame N+1).
# Then transicao_parar.py makes agachado_parar (walking crouched into the knee).
#
#   blender -b <x_animada.blend> --python agachar_completo.py -- [parado] [frames 10]
import bpy, os, sys
from mathutils import Quaternion, Vector
A = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
IDLE, N = (A + ["parado", "10"][len(A):])[:2]
N = int(N)
EASE_IN = 6
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe_no_chao

arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
down, walk, idle = bpy.data.actions["agachar_descer"], bpy.data.actions["andar_agachado"], bpy.data.actions[IDLE]
ease = lambda x: x * x * (3 - 2 * x)


def curves(action):
    by = {}
    for fc in action.fcurves:
        by.setdefault(fc.data_path, {})[fc.array_index] = fc
    return by


def sample(action, path, size, frame):
    got = curves.cache.setdefault(action.name, curves(action)).get(path, {})
    rest = (1.0, 0.0, 0.0, 0.0) if size == 4 else (1.0, 1.0, 1.0) if path.endswith("scale") else (0.0, 0.0, 0.0)
    return [got[i].evaluate(frame) if i in got else rest[i] for i in range(size)]
curves.cache = {}


def mix(a, b, w, size):
    if size == 4:
        qa, qb = Quaternion(a), Quaternion(b)
        if qa.dot(qb) < 0: qb.negate()
        return list(qa.slerp(qb, w))
    return [x + (y - x) * w for x, y in zip(a, b)]


PATHS = sorted({fc.data_path for act in (down, walk, idle) for fc in act.fcurves})


def make(name, frames):
    """An action from `frames`: a function (path, size) -> list of values, one per frame."""
    if name in bpy.data.actions:
        bpy.data.actions.remove(bpy.data.actions[name])
    curves.cache.pop(name, None)
    act = bpy.data.actions.new(name); act.use_fake_user = True
    for path in PATHS:
        size = 4 if path.endswith("rotation_quaternion") else 3
        group = path.split('"')[1] if '"' in path else None
        made = [act.fcurves.new(path, index=i, action_group=group) for i in range(size)]
        for f, v in enumerate(frames(path, size), start=1):
            for fc, x in zip(made, v):
                fc.keyframe_points.insert(f, x).interpolation = 'LINEAR'
    return act


d0, d1 = int(down.frame_range[0]), int(down.frame_range[1])
i0 = idle.frame_range[0]
w0, w1 = int(walk.frame_range[0]), int(walk.frame_range[1])

# andar_agachado centred: its hips' average side-to-side and back-and-forth taken out (world x, y), as a shift of
# the hips' own location.
hips = [b for b in arm.pose.bones if b.name.endswith("Hips")][0]
mw = arm.matrix_world
to_local = (mw.to_3x3() @ hips.bone.matrix_local.to_3x3()).inverted()
arm.animation_data.action = walk
xs, ys = [], []
for f in range(w0, w1 + 1):
    bpy.context.scene.frame_set(f); bpy.context.view_layer.update()
    h = mw @ hips.head; xs.append(h.x); ys.append(h.y)
shift = to_local @ Vector((-sum(xs) / len(xs), -sum(ys) / len(ys), 0))
for fc in walk.fcurves:
    if fc.data_path == 'pose.bones["%s"].location' % hips.name:
        for p in fc.keyframe_points:
            p.co[1] += shift[fc.array_index]; p.handle_left[1] += shift[fc.array_index]; p.handle_right[1] += shift[fc.array_index]
curves.cache.pop(walk.name, None)
print("CENTRED andar_agachado by %.3f m sideways, %.3f m back and forth" % (sum(xs) / len(xs), sum(ys) / len(ys)))


def crouch(path, size):
    stand = sample(idle, path, size, i0)
    out = []
    for f in range(d0, d1 + 1):                       # down
        v = sample(down, path, size, f)
        k = f - d0
        out.append(mix(stand, v, ease(k / EASE_IN), size) if k < EASE_IN else v)
    for j, f in enumerate(range(d1, d0 - 1, -1)):      # up: the same backwards
        v = sample(down, path, size, f)
        left = d1 - d0 - j
        out.append(mix(stand, v, ease(left / EASE_IN), size) if left < EASE_IN else v)
    return out


make("agachar", crouch)
knee = lambda path, size: [sample(down, path, size, d1)] * 2
make("agachado", knee)


def start(path, size):
    k = sample(down, path, size, d1)
    return [mix(k, sample(walk, path, size, w0 + i), ease(i / (N - 1)), size) for i in range(N)]


make("agachado_andar", start)
pe_no_chao.pin(arm, ["agachar", "agachado", "andar_agachado", "agachado_andar"])
length = (d1 - d0 + 1) * 2
print("MADE agachar %d frames (down by %d, up by %d), agachado, andar_agachado %d frames, agachado_andar %d frames"
      % (length, d1 - d0 + 1, length, w1 - w0, N))
arm.animation_data.action = bpy.data.actions.get("andar"); bpy.context.scene.frame_set(1)
bpy.ops.wm.save_mainfile()
