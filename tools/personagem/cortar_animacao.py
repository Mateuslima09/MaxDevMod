# WestPlay: an animation cut to its frames FIRST..LAST, starting at 1 (docs/personagem/INCLUIR_PERSONAGEM.md).
# For a jump that should go at once as Space is pressed (no slow bending before), as the running one does: its
# animation starts from its first frame, nothing to send it on to (owner 2026-11-06).
#   blender -b <x_animada.blend> --python cortar_animacao.py -- pular 24 55
import bpy, sys
A = sys.argv[sys.argv.index('--') + 1:]
NAME, FIRST, LAST = A[0], int(A[1]), int(A[2])
act = bpy.data.actions[NAME]
f0, f1 = act.frame_range
for fc in act.fcurves:
    vals = [(f, fc.evaluate(f)) for f in range(FIRST, LAST + 1)]
    fc.keyframe_points.clear()
    for f, v in vals:
        fc.keyframe_points.insert(f - FIRST + 1, v).interpolation = 'LINEAR'
print("CUT %s: frames %d-%d of %d-%d, now 1-%d" % (NAME, FIRST, LAST, f0, f1, LAST - FIRST + 1))
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
arm.animation_data.action = bpy.data.actions.get("andar"); bpy.context.scene.frame_set(1)
bpy.ops.wm.save_mainfile()
