# WestPlay: exports one person (or its headless body, or a variant with a second material such as the phone) to
# SCS's source formats, headless, with SCS Blender Tools 2.4.1 in Blender 3.6
# (docs/personagem/INCLUIR_PERSONAGEM.md, step 5). Then SCS's converter makes the game's files.
#
#   blender -b <x_animada.blend> --python exportar_scs.py -- <mesh> <dir> <model> "<anims>" [<skeleton model>] [headless]
#   e.g. the person:        -- mulher woman wm_player "walk:andar,run:correr,idle:parado"
#        its headless body: -- mulher woman wm_body "" wm_player headless
#        with the phone:    -- mulher woman wm_phone "" wm_player        (from the scene with the phone joined)
#
# <dir> is the folder under model/westplay/, <model> the file name (prefix_player / prefix_body / prefix_phone /
# prefix_phbody), "<anims>" game_name:action pairs (empty for a model that uses another's skeleton and animations).
# The SCS project folder: WESTPLAY_SCS_BASE, or personagem_scs\base beside the repo.
import bpy, math, os, sys, traceback
A=sys.argv[sys.argv.index("--")+1:]
MESH, DIR, MODEL = A[0], A[1], A[2]
ANIMS=[tuple(x.split(":")) for x in A[3].split(",") if x]
TEX=A[4] if len(A)>4 else MODEL
HEADLESS=len(A)>5 and A[5]=="headless"
BASE=os.path.normpath(os.environ.get("WESTPLAY_SCS_BASE") or os.path.join(os.path.dirname(os.path.abspath(__file__)),"..","..","..","personagem_scs","base"))
import addon_utils
addon_utils.enable("io_scs_tools", default_set=True)
from io_scs_tools.utils import get_scs_globals
from io_scs_tools.utils import material as mu
g=get_scs_globals()
g.scs_project_path=BASE
print("PROJECT", g.scs_project_path)
arm=bpy.data.objects["esqueleto"]; mesh=bpy.data.objects[MESH]
# drop preview helpers
for n in ["sol","camera"]:
    if n in bpy.data.objects: bpy.data.objects.remove(bpy.data.objects[n])
arm.animation_data.action=None
bpy.context.scene.frame_set(1)
# headless: the vertices that follow the head most (face, hair, cap, glasses) taken away
if HEADLESS:
    import bmesh
    heads={g.index for g in mesh.vertex_groups if g.name.split(":")[-1] in ("Head","HeadTop_End","Neck")}
    bm=bmesh.new(); bm.from_mesh(mesh.data); dl=bm.verts.layers.deform.active
    gone=[]
    for v in bm.verts:
        w=v[dl]
        if w and max(w.items(), key=lambda kv: kv[1])[0] in heads: gone.append(v)
    bmesh.ops.delete(bm, geom=gone, context='VERTS')
    bm.to_mesh(mesh.data); bm.free(); mesh.data.update()
    print("HEADLESS removed", len(gone), "verts; left", len(mesh.data.vertices))
# bone names: <=12 chars, [a-z0-9_]
import re
for b in arm.data.bones:
    n=b.name.split(":")[-1]
    n=n.replace("HandThumb","thumb").replace("HandIndex","index").replace("HandMiddle","middle").replace("HandRing","ring").replace("HandPinky","pinky")
    n=n.replace("Left","l_").replace("Right","r_").replace("Fore","fore").replace("UpLeg","upleg").replace("ToeBase","toe").replace("Toe_End","toe_e").replace("HeadTop_End","head_e")
    n=re.sub(r"[^a-z0-9_]","",n.lower())[:12]
    old=b.name; b.name=n
    for act in bpy.data.actions:
        for fc in act.fcurves:
            fc.data_path=fc.data_path.replace('"%s"'%old,'"%s"'%n)
        for gr in act.groups:
            if gr.name==old: gr.name=n
names=[b.name for b in arm.data.bones]; print("BONES",names); assert len(set(names))==len(names) and all(len(x)<=12 for x in names)
print("VGROUPS",[v.name for v in mesh.vertex_groups][:6])
print("FCURVE",bpy.data.actions["andar"].fcurves[0].data_path)
from io_scs_tools.exp import tobj as _tobj
if TEX==MODEL: print("TOBJ", _tobj.export(os.path.join(BASE,"model","westplay",DIR,MODEL+".tobj"), MODEL+".tga", set()))
# SCS root
bpy.ops.object.select_all(action='DESELECT')
for o in (arm,mesh): o.select_set(True)
bpy.context.view_layer.objects.active=arm
bpy.context.scene.cursor.location=(0,0,0)
bpy.ops.object.scs_tools_create_scs_root(scs_root_object_string=MODEL)
root=bpy.data.objects[MODEL]
import mathutils
arm.matrix_world = mathutils.Matrix.Rotation(math.pi,4,"Z") @ arm.matrix_world   # face game -Z
root.scs_props.scs_root_animated='anim'
root.scs_props.scs_root_object_allow_anim_custom_path=True
root.scs_props.scs_root_object_anim_export_filepath="//model/westplay/"+DIR+"/anim"
print("CHILDREN",[c.name for c in root.children], "mesh parent", mesh.parent.name if mesh.parent else None)
# material
mat=mesh.data.materials[0]
mat.name=MODEL
preset_name, section = mu.find_preset("eut2.dif.spec", {})
print("PRESET", preset_name)
mat.scs_props.active_shader_preset_name = "dif.spec"
mu.set_shader_data_to_material(mat, section)
mat.scs_props.mat_effect_name="eut2.dif.spec"
mat.scs_props.shader_texture_base = "//model/westplay/"+DIR+"/"+TEX+".tobj"
uvs=mat.scs_props.shader_texture_base_uv
if len(uvs)==0: uvs.add()
uvs[0].value=mesh.data.uv_layers[0].name
print("UV", [u.value for u in uvs])
mat.scs_props.shader_attribute_specular=(0.1,0.1,0.1)
mat.scs_props.shader_attribute_shininess=10.0
print("EFFECT", mat.scs_props.mat_effect_name, "TEX", mat.scs_props.shader_texture_base)
# a second material (the phone's): its own small texture beside the person's
for m2 in mesh.data.materials[1:]:
    m2.name=MODEL+"_"+m2.name
    m2.scs_props.active_shader_preset_name = "dif.spec"
    mu.set_shader_data_to_material(m2, section)
    m2.scs_props.mat_effect_name="eut2.dif.spec"
    m2.scs_props.shader_texture_base = "//model/westplay/"+DIR+"/"+MODEL.split("_")[0]+"_phone_tex.tobj"
    u2=m2.scs_props.shader_texture_base_uv
    if len(u2)==0: u2.add()
    u2[0].value=mesh.data.uv_layers[0].name
    m2.scs_props.shader_attribute_specular=(0.5,0.5,0.5)
    m2.scs_props.shader_attribute_shininess=60.0
    print("MATERIAL2", m2.name)
# animations
fps=30.0
for nm,src in ANIMS:
    bpy.context.view_layer.objects.active=root
    bpy.ops.object.scs_tools_add_animation()
    an=root.scs_object_animation_inventory[-1]
    act=bpy.data.actions[src]
    an.name=nm; an.action=src; an.anim_start=int(act.frame_range[0]); an.anim_end=int(act.frame_range[1])
    an.length=(an.anim_end-an.anim_start)/fps
    print("ANIM", nm, an.action, an.anim_start, an.anim_end, round(an.length,3))
arm.animation_data.action=bpy.data.actions[ANIMS[0][1]] if ANIMS else None
bpy.context.scene.scs_props.default_export_filepath="//model/westplay/"+DIR
bpy.ops.wm.save_as_mainfile(filepath=os.path.join(os.path.dirname(BASE),MODEL+".blend"))
# export
g.export_scope="scenes"
try:
    from io_scs_tools import exp as _export
    from io_scs_tools.utils import printout as _po
    _po.immediate_messenger.report_message_and_redraw = lambda *a, **k: None
    class Op:
        def report(self,t,m): print("REPORT",t,m)
    arm.data.pose_position="REST"; bpy.context.view_layer.update()
    objs=[root]+list(root.children_recursive)
    r=_export.batch_export(Op(), objs)
    print("EXPORT", r)
except Exception:
    traceback.print_exc()
# The headless body on the full model's skeleton: the animations name theirs (`Skeleton: "../<full>.pis"`), and
# the game plays them only on a model of the same skeleton (the same bones under another name stayed still).
if TEX != MODEL:
    d=os.path.join(BASE,"model","westplay",DIR)
    pim=os.path.join(d,MODEL+".pim")
    text=open(pim,encoding="utf-8").read().replace('Skeleton: "%s.pis"'%MODEL, 'Skeleton: "%s.pis"'%TEX)
    open(pim,"w",encoding="utf-8").write(text)
    own=os.path.join(d,MODEL+".pis")
    if os.path.exists(own): os.remove(own)
    print("SKELETON", MODEL, "->", TEX)
# The animations name their skeleton one folder up (a custom anim folder makes the exporter write it beside them).
import glob
for p in glob.glob(os.path.join(BASE,"model","westplay",DIR,"anim","*.pia")):
    t=open(p,encoding="utf-8").read(); t2=t.replace('Skeleton: "%s.pis"'%MODEL,'Skeleton: "../%s.pis"'%MODEL)
    if t2!=t:
        open(p,"w",encoding="utf-8").write(t2); print("PIA skeleton ../%s.pis:"%MODEL, os.path.basename(p))
