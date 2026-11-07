# WestPlay: one person's Mixamo animations in one Blender scene, at the standard height, with the standard names
# (docs/personagem/INCLUIR_PERSONAGEM.md, step 3).
#
#   blender -b --factory-startup --python juntar_personagem.py -- <folder> <homem|mulher> <name> <out.blend> \
#       "andar=Walking.fbx;correr=Running.fbx;parado=Idle.fbx;..."
#
# - The first file gives the model and the skeleton; from the others only their animation is kept.
# - The standard height: the Head bone (Mixamo puts it where the neck meets the head, hats and hair not counted)
#   at HEAD[kind] metres; the model, its skeleton and every animation's moves scaled together to it.
# - Stopping ("parar") held in place (its hips' drift taken out); the jumps ("pular", "pular_correndo") with the
#   lowest foot on the floor each frame (pe_no_chao.py: the game's physics lifts the body).
import bpy, os, sys
from mathutils import Vector
A = sys.argv[sys.argv.index("--") + 1:]
FOLDER, KIND, NAME, OUT, LIST = A[0], A[1], A[2], A[3], A[4]
# The standard (the first man and woman, 2026-11-06): Head bone 1.52 m (1.80 m tall), 1.46 m (1.68 m tall).
HEAD = {"homem": 1.52, "mulher": 1.46}
# The names the export (exportar_scs.py) and the game (own_character.rs ANIMATIONS) know them by.
STANDARD = {"andar": "walk", "correr": "run", "parado": "idle", "parar": "stop", "celular": "text",
            "celular_andando": "txwalk", "pular": "jump", "pular_correndo": "jumprun", "agachar": "crouch",
            # only the model and its skeleton (Mixamo's T-pose download), the animations reused (reaproveitar_animacoes.py)
            "modelo": None}
files = [tuple(x.split("=", 1)) for x in LIST.split(";") if x.strip()]
for name, _ in files:
    assert name in STANDARD, f"unknown animation name {name!r}: one of {sorted(STANDARD)}"
assert files[0][0] in ("andar", "modelo"), "the first file is the walk (andar) or the model alone (modelo)"
bpy.ops.wm.read_factory_settings(use_empty=True)
arm = mesh = None
for name, file in files:
    before = set(bpy.data.objects)
    bpy.ops.import_scene.fbx(filepath=os.path.join(FOLDER, file))
    new = [o for o in bpy.data.objects if o not in before]
    a = [o for o in new if o.type == 'ARMATURE'][0]
    act = a.animation_data.action if a.animation_data else None
    if name == "modelo":
        if act: bpy.data.actions.remove(act)
    else:
        act.name = name; act.use_fake_user = True
    if arm is None:
        arm = a; arm.name = "esqueleto"
        # the skinned mesh (a stray one, like a leftover cube in the upload, taken out)
        skinned = [o for o in new if o.type == 'MESH' and any(m.type == 'ARMATURE' for m in o.modifiers)]
        mesh = skinned[0]; mesh.name = NAME
        for o in new:
            if o.type != 'ARMATURE' and o not in skinned: bpy.data.objects.remove(o, do_unlink=True)
    else:
        for o in new: bpy.data.objects.remove(o, do_unlink=True)
for coll in (bpy.data.meshes, bpy.data.armatures, bpy.data.materials, bpy.data.images):
    for x in list(coll):
        if x.users == 0: coll.remove(x)
sc = bpy.context.scene; sc.render.fps = 30
# --- the standard height
if arm.animation_data: arm.animation_data.action = None
arm.data.pose_position = 'REST'; bpy.context.view_layer.update()
head = [b for b in arm.pose.bones if b.name.endswith("Head")][0]
k = HEAD[KIND] / (arm.matrix_world @ head.head).z
print("HEIGHT head bone %.3f m, scaled by %.4f" % ((arm.matrix_world @ head.head).z, k))
s = arm.scale.x * k
arm.scale = (s, s, s)
for o in bpy.data.objects: o.select_set(o in (arm, mesh))
bpy.context.view_layer.objects.active = arm
bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
# the moves (bone locations) were in the old units
for act in bpy.data.actions:
    for fc in act.fcurves:
        if fc.data_path.endswith(".location"):
            for p in fc.keyframe_points:
                p.co[1] *= s; p.handle_left[1] *= s; p.handle_right[1] *= s
bpy.context.view_layer.update()
dg = bpy.context.evaluated_depsgraph_get(); e = mesh.evaluated_get(dg); me = e.to_mesh()
zs = [(mesh.matrix_world @ v.co).z for v in me.vertices]; e.to_mesh_clear()
print("HEIGHT now: head bone %.3f m, model %.3f m tall (hat and hair counted), floor %.3f"
      % ((arm.matrix_world @ head.head).z, max(zs) - min(zs), min(zs)))
arm.data.pose_position = 'POSE'
# --- stopping, in place: the hips' channels that drift away held at their first value
if "parar" in bpy.data.actions:
    for fc in bpy.data.actions["parar"].fcurves:
        if fc.data_path.endswith('Hips"].location'):
            ks = fc.keyframe_points; first = ks[0].co[1]
            if abs(ks[-1].co[1] - first) > 0.1:  # metres: the steps forward, not the hips going down
                for p in ks: p.co[1] = first; p.handle_left[1] = first; p.handle_right[1] = first
                print("STOP held in place: hips channel", fc.array_index)
# --- the jumps, the lowest foot on the floor
jumps = [n for n in ("pular", "pular_correndo") if n in bpy.data.actions]
if jumps:
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import pe_no_chao
    pe_no_chao.pin(arm, jumps)
bpy.ops.file.pack_all()
if arm.animation_data: arm.animation_data.action = bpy.data.actions.get("andar")
sc.frame_set(1)
bpy.ops.wm.save_as_mainfile(filepath=OUT)
print("ACTIONS", [(a.name, STANDARD.get(a.name), tuple(int(x) for x in a.frame_range)) for a in bpy.data.actions])
