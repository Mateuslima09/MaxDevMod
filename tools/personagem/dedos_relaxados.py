# WestPlay: fingers at rest in animations that have none (docs/personagem/INCLUIR_PERSONAGEM.md, "Reaproveitar").
# An animation from a "No Fingers" skeleton leaves this one's fingers straight, as in its T-pose; they're given the
# pose they have in another animation's frame (relaxed), held all through.
#   blender -b <this_animada.blend> --python dedos_relaxados.py -- <action with fingers> <frame> andar,correr,parado
import bpy, sys
A = sys.argv[sys.argv.index('--') + 1:]
SOURCE, FRAME, TARGETS = A[0], int(A[1]), A[2].split(",")
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]; sc = bpy.context.scene
fingers = [b for b in arm.pose.bones if "Thumb" in b.name or "Index" in b.name or "Middle" in b.name
           or "Ring" in b.name or "Pinky" in b.name]
arm.animation_data.action = bpy.data.actions[SOURCE]; sc.frame_set(FRAME)
pose = {b.name: b.rotation_quaternion.copy() for b in fingers}
for name in TARGETS:
    act = bpy.data.actions[name]; f0, f1 = act.frame_range
    have = {fc.data_path.split('"')[1] for fc in act.fcurves if fc.data_path.startswith("pose.bones")}
    added = 0
    for b in fingers:
        if b.name in have: continue
        path = 'pose.bones["%s"].rotation_quaternion' % b.name
        for i in range(4):
            fc = act.fcurves.new(path, index=i, action_group=b.name)
            for f in (f0, f1):
                fc.keyframe_points.insert(f, pose[b.name][i]).interpolation = 'LINEAR'
        added += 1
    print("FINGERS", name, "relaxed:", added, "bones")
arm.animation_data.action = bpy.data.actions.get("andar"); sc.frame_set(1)
bpy.ops.wm.save_mainfile()
