# WestPlay: exporta animações diretamente de um .blend SCS (ex: wg_player.blend) para arquivos .pia
# bit-compatíveis com o SCS Conversion Tools (resconvert.exe), sem depender do addon io_scs_tools.
#
#   blender -b wg_player.blend --python exportar_pia.py -- <pasta_anim> <skeleton_rel> "walk:andar,idle:parado,..."
#
import bpy, os, re, struct, sys
from mathutils import Matrix

A = sys.argv[sys.argv.index('--') + 1:]
ANIM_DIR = A[0]
SKEL_REL = A[1]
PAIRS = [tuple(x.split(':')) for x in A[2].split(',') if x]

FPS = 30.0
DT = 1.0 / FPS

def f_to_hex(v):
    if abs(v) < 1e-12:
        v = 0.0
    return '&' + struct.pack('>f', float(v)).hex()

# Blender -> SCS conversion matrix (X->X, Z->Y, -Y->Z)
C = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))

arm = bpy.data.objects["esqueleto"]
roots = [o for o in bpy.data.objects if o.parent is None and o.type == 'EMPTY']
root = roots[0] if roots else None
rel = (root.matrix_world.inverted() @ arm.matrix_world) if root else arm.matrix_world

# Read bone order from an existing .pia in ANIM_DIR if present, else use all pose bones with fcurves
def get_bone_names(pia_path, act):
    if os.path.exists(pia_path):
        with open(pia_path, 'r', encoding='utf-8') as f:
            txt = f.read()
        names = re.findall(r'BoneChannel\s*\{\s*Name:\s*"([^"]+)"', txt)
        if names:
            return names
    # fallback: bones present in action fcurves in armature order
    present = {fc.data_path.split('"')[1] for fc in act.fcurves if fc.data_path.startswith('pose.bones["')}
    return [b.name for b in arm.pose.bones if b.name in present]

sc = bpy.context.scene

for scs_name, act_name in PAIRS:
    act = bpy.data.actions.get(act_name)
    if not act:
        print(f"AVISO: action '{act_name}' não encontrada, pulando {scs_name}.pia")
        continue
    pia_path = os.path.join(ANIM_DIR, f"{scs_name}.pia")
    bones = get_bone_names(pia_path, act)

    f0 = int(act.frame_range[0])
    f1 = int(act.frame_range[1])
    n_frames = f1 - f0 + 1
    total_time = (f1 - f0) / FPS

    arm.animation_data.action = act
    # Collect per-bone matrices across all frames
    bone_mats = {b: [] for b in bones}
    for f in range(f0, f1 + 1):
        sc.frame_set(f)
        bpy.context.view_layer.update()
        # Cache world-in-SCS matrix per bone
        scs_world = {}
        for pb in arm.pose.bones:
            scs_world[pb.name] = C @ rel @ pb.matrix
        for bname in bones:
            pb = arm.pose.bones[bname]
            if pb.parent is None:
                m = scs_world[bname]
            else:
                m = scs_world[pb.parent.name].inverted() @ scs_world[bname]
            bone_mats[bname].append(m.transposed())

    dt_hex = f_to_hex(DT)
    lines = [
        "Header {",
        "    FormatVersion: 3",
        '    Source: "Blender 3.6.23, WestPlay exportar_pia.py"',
        '    Type: "Animation"',
        f'    Name: "{scs_name}"',
        "}",
        "Global {",
        f'    Skeleton: "{SKEL_REL}"',
        f"    TotalTime: {total_time:.16g}",
        f"    BoneChannelCount: {len(bones)}",
        "    CustomChannelCount: 0",
        "}",
    ]
    for bname in bones:
        lines.append("BoneChannel {")
        lines.append(f'    Name: "{bname}"')
        lines.append("    StreamCount: 2")
        lines.append(f"    KeyframeCount: {n_frames}")
        lines.append("    Stream {")
        lines.append("        Format: FLOAT")
        lines.append('        Tag: "_TIME"')
        for i in range(n_frames):
            lines.append(f"        {i:<4d} ( {dt_hex} )")
        lines.append("    }")
        lines.append("    Stream {")
        lines.append("        Format: FLOAT4x4")
        lines.append('        Tag: "_MATRIX"')
        for i, mt in enumerate(bone_mats[bname]):
            r0 = "  ".join(f_to_hex(mt[0][c]) for c in range(4))
            r1 = "  ".join(f_to_hex(mt[1][c]) for c in range(4))
            r2 = "  ".join(f_to_hex(mt[2][c]) for c in range(4))
            r3 = "  ".join(f_to_hex(mt[3][c]) for c in range(4))
            lines.append(f"        {i:<4d} (  {r0} ")
            lines.append(f"                {r1}")
            lines.append(f"                {r2}")
            lines.append(f"                {r3} )")
        lines.append("    }")
        lines.append("}")

    with open(pia_path, 'w', encoding='utf-8', newline='\n') as f:
        f.write("\n".join(lines) + "\n")
    print(f"EXPORTED {pia_path} ({n_frames} frames, {len(bones)} bones, {total_time:.3f}s)")
