# WestPlay: a "3 Chain Fingers" skeleton (thumb, index and ring: 49 bones) given animations from a "2 Chain
# Fingers" one (41: thumb and index, reaproveitar_animacoes.py): its ring fingers would stay straight at rest, so
# each ring bone gets its index bone's turns (the cowboy, 2026-11-06).
#   blender -b <x_animada.blend> --python anelar_como_indicador.py
import bpy
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
bones = {b.name for b in arm.data.bones}
done = 0
for act in bpy.data.actions:
    for side in ("Left", "Right"):
        for i in range(1, 5):
            src = next((b for b in bones if b.endswith(f"{side}HandIndex{i}")), None)
            dst = next((b for b in bones if b.endswith(f"{side}HandRing{i}")), None)
            if not src or not dst:
                continue
            for fc in [f for f in act.fcurves if f.data_path == f'pose.bones["{dst}"].rotation_quaternion']:
                act.fcurves.remove(fc)
            for fc in [f for f in act.fcurves if f.data_path == f'pose.bones["{src}"].rotation_quaternion']:
                new = act.fcurves.new(f'pose.bones["{dst}"].rotation_quaternion', index=fc.array_index, action_group=dst)
                new.keyframe_points.add(len(fc.keyframe_points))
                for p, q in zip(fc.keyframe_points, new.keyframe_points):
                    q.co = p.co; q.interpolation = p.interpolation
                    q.handle_left = p.handle_left; q.handle_right = p.handle_right
                done += 1
print("RING from index: %d curves" % done)
bpy.ops.wm.save_mainfile()
