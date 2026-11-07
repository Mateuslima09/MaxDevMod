# WestPlay: walking into standing still with no jump of pose (docs/personagem/TRANSICOES.md, stage 2).
# The SCS engine plays one animation per person and blends none, so the change is made here, beforehand: one
# animation of PIECES pieces, each starting from the walk's pose at a piece of its cycle (held: no more steps)
# and turning, a frame at a time, into standing's first frame. The plugin plays the piece nearest to where the
# walk was as it stopped, then standing from its first frame.
#
# Not the walk going on into standing (the first try, 2026-11-06): with the body already still, its steps went
# on for up to a second, "two, three steps, as if sliding" (owner). Held, the feet only close in, in 0.3 s.
#
#   blender -b <x_animada.blend> --python transicao_parar.py -- [andar] [parado] [action andar_parar] [frames 10]
#
# Makes the action <name> (the game's name: settle), PIECES x frames long, the lowest foot on the floor each
# frame (pe_no_chao.py), and saves the scene.
import bpy, os, sys
from mathutils import Quaternion
A = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
WALK, IDLE, NAME, FRAMES = (A + ["andar", "parado", "andar_parar", "10"][len(A):])[:4]
FRAMES = int(FRAMES)
PIECES = 8

arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
walk, idle = bpy.data.actions[WALK], bpy.data.actions[IDLE]
w0, w1 = walk.frame_range
cycle = w1 - w0            # frames: the last is the first again (Mixamo's loop), as the game plays it
i0 = idle.frame_range[0]


def curves(action):
    by = {}
    for fc in action.fcurves:
        by.setdefault(fc.data_path, {})[fc.array_index] = fc
    return by


walks, idles = curves(walk), curves(idle)


def value(by, path, size, frame):
    got = by.get(path, {})
    rest = (1.0, 0.0, 0.0, 0.0) if path.endswith("rotation_quaternion") else (1.0, 1.0, 1.0) if path.endswith("scale") else (0.0, 0.0, 0.0)
    return [got[i].evaluate(frame) if i in got else rest[i] for i in range(size)]


ease = lambda x: x * x * (3 - 2 * x)   # smoothstep: no jolt as it starts nor as it ends
for old in [a for a in bpy.data.actions if a.name == NAME or a.name.startswith(NAME) and a.name[len(NAME):].isdigit()]:
    bpy.data.actions.remove(old)
act = bpy.data.actions.new(NAME); act.use_fake_user = True
for path in sorted(set(walks) | set(idles)):
    size = 4 if path.endswith("rotation_quaternion") else 3
    group = path.split('"')[1] if '"' in path else None
    made = [act.fcurves.new(path, index=i, action_group=group) for i in range(size)]
    for piece in range(PIECES):
        a = value(walks, path, size, w0 + piece * cycle / PIECES)    # the walk's pose, held
        b = value(idles, path, size, i0)
        for i in range(FRAMES):
            w = ease(i / (FRAMES - 1))
            if size == 4:
                qa, qb = Quaternion(a), Quaternion(b)
                if qa.dot(qb) < 0: qb.negate()
                v = list(qa.slerp(qb, w))
            else:
                v = [x + (y - x) * w for x, y in zip(a, b)]
            for fc, x in zip(made, v):
                fc.keyframe_points.insert(1 + piece * FRAMES + i, x).interpolation = 'LINEAR'
for piece in range(PIECES):
    print("PIECE %d: frames %d-%d, from the walk's frame %.2f into standing's frame %d"
          % (piece, 1 + piece * FRAMES, (piece + 1) * FRAMES, w0 + piece * cycle / PIECES, i0))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe_no_chao
pe_no_chao.pin(arm, [NAME])
arm.animation_data.action = bpy.data.actions.get("andar"); bpy.context.scene.frame_set(1)
bpy.ops.wm.save_mainfile()
