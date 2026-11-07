# WestPlay: another person's animations on this one (docs/personagem/INCLUIR_PERSONAGEM.md, "Reaproveitar").
# Mixamo's skeletons name their bones the same and turn them the same way, so an animation's turns carry over bone
# by bone; only the hips' moves are scaled, by how high each person's hips are. A bone the other one hasn't (the
# fingers on a "No Fingers" skeleton) is left out.
#
#   blender -b <this_animada.blend> --python reaproveitar_animacoes.py -- <other_animada.blend> "pular,agachar=agachar"
#
# Each item is an action of the other scene, or other=this to name it here; one already here is replaced.
# Then each one with its lowest foot on the floor, every frame (the legs are not the same length).
import bpy, sys
A = sys.argv[sys.argv.index('--') + 1:]
SOURCE, WANTED = A[0], [tuple((x.split("=") + [x])[:2]) for x in A[1].split(",") if x]
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
mine = {b.name for b in arm.data.bones}
def hips_height(armature):
    b = [x for x in armature.data.bones if x.name.endswith("Hips")][0]
    return (armature.matrix_world @ b.head_local).z
here = hips_height(arm)
before = set(bpy.data.actions)
with bpy.data.libraries.load(SOURCE, link=False) as (src, dst):
    dst.objects = [n for n in src.objects]
    dst.actions = [n for n, _ in WANTED]
other = [o for o in dst.objects if o is not None and o.type == 'ARMATURE'][0]
k = here / hips_height(other)
print("REUSE from", SOURCE, "hips %.3f -> %.3f (x%.3f)" % (hips_height(other), here, k))
for (theirs, name), act in zip(WANTED, dst.actions):
    if name in bpy.data.actions and bpy.data.actions[name] is not act:
        bpy.data.actions.remove(bpy.data.actions[name])
    act.name = name; act.use_fake_user = True
    dropped = set()
    for fc in list(act.fcurves):
        bone = fc.data_path.split('"')[1] if fc.data_path.startswith("pose.bones") else None
        if bone is None or bone not in mine:
            dropped.add(bone); act.fcurves.remove(fc); continue
        if fc.data_path.endswith(".location"):
            if bone.endswith("Hips"):
                for p in fc.keyframe_points:
                    p.co[1] *= k; p.handle_left[1] *= k; p.handle_right[1] *= k
            else:
                act.fcurves.remove(fc)          # the other's bone lengths: here they're this one's
    print("ACTION", name, "from", theirs, tuple(int(x) for x in act.frame_range), "left out:", sorted(b for b in dropped if b)[:6])
for o in dst.objects:
    if o is not None: bpy.data.objects.remove(o, do_unlink=True)
# the other scene's own actions that came along with its skeleton
for x in [a for a in bpy.data.actions if a not in before and a not in dst.actions]:
    bpy.data.actions.remove(x)
for coll in (bpy.data.meshes, bpy.data.armatures, bpy.data.materials, bpy.data.images):
    for x in list(coll):
        if x.users == 0: coll.remove(x)
# the other's legs are not this one's length: the lowest foot back on the floor each frame (pe_no_chao.py;
# in every one of these a foot is on the ground but in the jumps' air, where the game's physics lifts the body)
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pe_no_chao
pe_no_chao.pin(arm, [name for _, name in WANTED], constant={"correr"})
bpy.ops.file.pack_all()
bpy.ops.wm.save_mainfile()
print("ACTIONS", sorted(a.name for a in bpy.data.actions))
