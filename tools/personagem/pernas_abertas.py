# WestPlay: the legs a little further apart in every animation, for a person whose legs are thicker than the one
# the animations came from (reaproveitar_animacoes.py): the cowboy's wide trousers and boots, on the tattooed man's
# narrow stance, met from the knee down and went through each other walking (owner 2026-11-06). Each thigh turned
# outward by DEGREES (about the forward axis, at the hip), the shin back parallel to where it was and the foot as it
# was: the lower leg moved out whole, by the thigh's length x sin(DEGREES). Then the lowest foot on the floor again.
#   blender -b <x_animada.blend> --python pernas_abertas.py -- <degrees> [action,action,...]
import bpy, math, os, sys
from mathutils import Matrix, Vector
A = sys.argv[sys.argv.index('--') + 1:]
DEG = float(A[0])
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
names = A[1].split(",") if len(A) > 1 and A[1] else [a.name for a in bpy.data.actions]
bone = lambda s: [b for b in arm.pose.bones if b.name.endswith(s)][0]
# The world's forward axis (the person faces -Y) in the armature's own space (Mixamo's comes turned 90 degrees).
AXIS = (arm.matrix_world.to_3x3().inverted() @ Vector((0, 1, 0))).normalized()
legs = [(bone(f"{s}UpLeg"), bone(f"{s}Leg"), bone(f"{s}Foot"), sign) for s, sign in (("Left", -1), ("Right", 1))]
for name in names:
    act = bpy.data.actions[name]
    arm.animation_data.action = act
    f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
    for f in range(f0, f1 + 1):
        bpy.context.scene.frame_set(f); bpy.context.view_layer.update()
        for up, leg, foot, sign in legs:
            leg_rot = leg.matrix.to_3x3().copy(); foot_rot = foot.matrix.to_3x3().copy()
            h = up.matrix.translation.copy()
            turn = Matrix.Translation(h) @ Matrix.Rotation(math.radians(DEG) * sign, 4, AXIS) @ Matrix.Translation(-h)
            up.matrix = turn @ up.matrix; bpy.context.view_layer.update()
            leg.matrix = Matrix.Translation(leg.matrix.translation) @ leg_rot.to_4x4(); bpy.context.view_layer.update()
            foot.matrix = Matrix.Translation(foot.matrix.translation) @ foot_rot.to_4x4(); bpy.context.view_layer.update()
            for b in (up, leg, foot):
                b.keyframe_insert("rotation_quaternion", frame=f, group=b.name)
    for fc in act.fcurves:
        for p in fc.keyframe_points: p.interpolation = 'LINEAR'
    print("WIDER %s: frames %d-%d, thighs out %.1f deg" % (name, f0, f1, DEG))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe_no_chao
pe_no_chao.pin(arm, names, constant={"correr"} & set(names))
arm.animation_data.action = bpy.data.actions.get("andar"); bpy.context.scene.frame_set(1)
bpy.ops.wm.save_mainfile()
