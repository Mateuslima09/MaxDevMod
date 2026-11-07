# WestPlay: an animation's end turned, over a few frames more, into standing's first frame (docs/personagem/
# TRANSICOES.md 7.14). The plugin then starts standing at its first frame as the animation ends: no jump of pose
# between them (owner 2026-11-06: after the jump, "a little hop up in the very last frame", its end bent and
# standing upright).
#   blender -b <x_animada.blend> --python emendar_no_parado.py -- pular [parado] [frames 8]
import bpy, os, sys
from mathutils import Quaternion
A = sys.argv[sys.argv.index('--') + 1:]
NAME, IDLE, FRAMES = (A + ["", "parado", "8"][len(A):])[:3]
FRAMES = int(FRAMES)
act, idle = bpy.data.actions[NAME], bpy.data.actions[IDLE]
end = int(act.frame_range[1]); i0 = idle.frame_range[0]
by = lambda action: {(fc.data_path, fc.array_index): fc for fc in action.fcurves}
mine, theirs = by(act), by(idle)
ease = lambda x: x * x * (3 - 2 * x)
paths = sorted({p for p, _ in mine} | {p for p, _ in theirs})
for path in paths:
    size = 4 if path.endswith("rotation_quaternion") else 3
    rest = (1.0, 0.0, 0.0, 0.0) if size == 4 else (1.0, 1.0, 1.0) if path.endswith("scale") else (0.0, 0.0, 0.0)
    a = [mine[(path, i)].evaluate(end) if (path, i) in mine else rest[i] for i in range(size)]
    b = [theirs[(path, i)].evaluate(i0) if (path, i) in theirs else rest[i] for i in range(size)]
    curves = [mine.get((path, i)) or act.fcurves.new(path, index=i, action_group=path.split('"')[1] if '"' in path else None)
              for i in range(size)]
    for k in range(1, FRAMES + 1):
        w = ease(k / FRAMES)
        if size == 4:
            qa, qb = Quaternion(a), Quaternion(b)
            if qa.dot(qb) < 0: qb.negate()
            v = list(qa.slerp(qb, w))
        else:
            v = [x + (y - x) * w for x, y in zip(a, b)]
        for fc, x in zip(curves, v):
            fc.keyframe_points.insert(end + k, x).interpolation = 'LINEAR'
print("JOINED %s: frames %d-%d into %s's frame %d (now 1-%d)" % (NAME, end + 1, end + FRAMES, IDLE, i0, end + FRAMES))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe_no_chao
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
pe_no_chao.pin(arm, [NAME])
arm.animation_data.action = bpy.data.actions.get("andar"); bpy.context.scene.frame_set(1)
bpy.ops.wm.save_mainfile()
