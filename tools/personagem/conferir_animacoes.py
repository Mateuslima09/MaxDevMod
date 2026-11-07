# WestPlay: checks one person's combined scene against the standard (docs/personagem/INCLUIR_PERSONAGEM.md,
# step 4) and says, in Portuguese, what's wrong:
#   blender -b <x_animada.blend> --python conferir_animacoes.py -- <homem|mulher>
import bpy, sys, math
KIND = sys.argv[sys.argv.index('--') + 1]
HEAD = {"homem": 1.52, "mulher": 1.46}[KIND]
LOOPS = {"andar", "correr", "parado", "celular", "celular_andando"}
ONCE = {"parar", "pular", "pular_correndo", "agachar"}
NEEDED = ["andar", "correr", "parado"]
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
meshes = [o for o in bpy.data.objects if o.type == 'MESH' and any(m.type == 'ARMATURE' for m in o.modifiers)]
sc = bpy.context.scene; mw = arm.matrix_world
bone = lambda n: [b for b in arm.pose.bones if b.name.endswith(n)][0]
problems = []; notes = []
def low():
    dg = bpy.context.evaluated_depsgraph_get(); z = 1e9
    for o in meshes:
        e = o.evaluated_get(dg); me = e.to_mesh()
        z = min([z] + [(o.matrix_world @ v.co).z for v in me.vertices]); e.to_mesh_clear()
    return z
if sc.render.fps != 30: problems.append(f"a cena está a {sc.render.fps} fps; o padrão é 30")
if len(meshes) != 1: problems.append(f"{len(meshes)} malhas com esqueleto; o padrão é uma só")
long_names = [b.name for b in arm.data.bones if not b.name.startswith("mixamorig:")]
if long_names: notes.append(f"ossos fora do padrão Mixamo: {long_names[:5]}")
# rest pose: height
keep = arm.animation_data.action if arm.animation_data else None
if arm.animation_data: arm.animation_data.action = None
arm.data.pose_position = 'REST'; bpy.context.view_layer.update()
floor = low(); head = (mw @ bone("Head").head).z
feet = [bone(n) for n in ("LeftFoot", "LeftToeBase", "LeftToe_End", "RightFoot", "RightToeBase", "RightToe_End")]
rest = [(mw @ f.head).z for f in feet]
print(f"ALTURA  osso da cabeça {head:.3f} m (padrão {HEAD} m), chão {floor:+.3f}")
if abs(head - HEAD) > 0.01: problems.append(f"altura: osso da cabeça a {head:.3f} m, o padrão é {HEAD} (rode juntar_personagem.py)")
if abs(floor) > 0.01: problems.append(f"os pés em repouso a {floor:+.3f} m do chão")
arm.data.pose_position = 'POSE'
if not arm.animation_data: arm.animation_data_create()
names = {a.name for a in bpy.data.actions}
for n in NEEDED:
    if n not in names: problems.append(f"falta a animação {n}")
for a in sorted(bpy.data.actions, key=lambda a: a.name):
    arm.animation_data.action = a; f0, f1 = int(a.frame_range[0]), int(a.frame_range[1])
    hips = []; foot = []
    for f in range(f0, f1 + 1):
        sc.frame_set(f); bpy.context.view_layer.update()
        hips.append(mw @ bone("Hips").head); foot.append(min((mw @ b.head).z - r for b, r in zip(feet, rest)))
    def pose(f):
        sc.frame_set(f); bpy.context.view_layer.update()
        return {b.name: b.matrix.to_quaternion() for b in arm.pose.bones}
    p0, p1 = pose(f0), pose(f1)
    seam = max(math.degrees(p0[n].rotation_difference(p1[n]).angle) for n in p0)
    lows = []
    for f in range(f0, f1 + 1, max(1, (f1 - f0) // 12)):
        sc.frame_set(f); bpy.context.view_layer.update(); lows.append(low())
    drift = max(max(h.x for h in hips) - min(h.x for h in hips), max(h.y for h in hips) - min(h.y for h in hips))
    line = (f"{a.name:16s} quadros {f0}-{f1} ({(f1 - f0) / 30:.2f} s)  emenda {seam:5.1f}°  quadril anda {drift:.2f} m  "
            f"pé mais baixo {min(foot):+.3f}..{max(foot):+.3f}  malha {min(lows):+.3f}")
    print("ANIM   ", line)
    if a.name not in LOOPS | ONCE: notes.append(f"{a.name}: nome fora do padrão (veja juntar_personagem.py)")
    if a.name in LOOPS and seam > 3: problems.append(f"{a.name}: o fim não emenda no começo ({seam:.0f}°): vai dar um tranco a cada volta")
    if drift > 0.25: problems.append(f"{a.name}: o quadril sai {drift:.2f} m do lugar: baixe do Mixamo com 'In Place'")
    if min(lows) < -0.03: problems.append(f"{a.name}: a malha atravessa o chão ({min(lows):+.3f} m)")
    if a.name in ("pular", "pular_correndo") and max(abs(min(foot)), abs(max(foot))) > 0.01:
        problems.append(f"{a.name}: o pé mais baixo não fica no chão ({min(foot):+.3f}..{max(foot):+.3f}): rode pe_no_chao.py")
    if a.name not in ("pular", "pular_correndo") and min(foot) > 0.03:
        problems.append(f"{a.name}: os pés nunca tocam o chão (o mais baixo a {min(foot):+.3f} m)")
if keep: arm.animation_data.action = keep
for n in notes: print("NOTA   ", n)
for p in problems: print("PROBLEMA", p)
print("RESULTADO", "ok" if not problems else f"{len(problems)} problema(s)")
