# WestPlay: smooth animations to remove robotic micro-jitter and harsh linear keyframes.
# Applies Gaussian filtering across quaternion rotations and bone locations, guarantees seamless
# boundary loops for cyclic actions (walk/idle/run), converts interpolation to BEZIER with AUTO_CLAMPED
# handles, and re-pins the lowest foot to the floor.
#
#   blender -b <scene.blend> --python suavizar_animacoes.py -- [action,action,...] [saida.blend] [radius]
#
import bpy, math, os, sys
from mathutils import Vector, Quaternion

A = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
ACTIONS = [x.strip() for x in A[0].split(',') if x.strip()] if len(A) > 0 and A[0] else ['andar', 'parado']
OUT = A[1] if len(A) > 1 and A[1] else None
RADIUS = int(A[2]) if len(A) > 2 and A[2] else 2

arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]

# Gaussian smoothing weights
if RADIUS == 1:
    weights = [1.0, 2.0, 1.0]
elif RADIUS == 2:
    weights = [1.0, 4.0, 6.0, 4.0, 1.0]
elif RADIUS == 3:
    weights = [1.0, 6.0, 15.0, 20.0, 15.0, 6.0, 1.0]
else:
    weights = [1.0]

total_w = sum(weights)
kernel = [w / total_w for w in weights]
k_rad = len(kernel) // 2

LOOPS = {'andar', 'parado', 'correr', 'andar_agachado', 'celular', 'celular_andando'}

for act_name in ACTIONS:
    act = bpy.data.actions.get(act_name)
    if not act:
        print(f"Action '{act_name}' not found, skipping.")
        continue

    f0 = int(act.frame_range[0])
    f1 = int(act.frame_range[1])
    n_frames = f1 - f0 + 1
    is_loop = act_name in LOOPS
    print(f"Suavizando '{act_name}': quadros {f0}..{f1} ({n_frames} frames, loop={is_loop})")

    quat_curves = {}
    loc_curves = {}

    for fc in act.fcurves:
        if 'rotation_quaternion' in fc.data_path:
            bone_name = fc.data_path.split('"')[1]
            quat_curves.setdefault(bone_name, {})[fc.array_index] = fc
        elif 'location' in fc.data_path:
            bone_name = fc.data_path.split('"')[1]
            loc_curves.setdefault(bone_name, {})[fc.array_index] = fc

    # Smooth Quaternions for animated bones
    for bone_name, channels in quat_curves.items():
        if len(channels) < 4:
            continue
        if len(channels[0].keyframe_points) != n_frames:
            continue

        quats = []
        for f in range(f0, f1 + 1):
            w = channels[0].evaluate(f)
            x = channels[1].evaluate(f)
            y = channels[2].evaluate(f)
            z = channels[3].evaluate(f)
            quats.append(Quaternion((w, x, y, z)))

        # Sign continuity (q and -q)
        for i in range(1, len(quats)):
            if quats[i].dot(quats[i-1]) < 0:
                quats[i].negate()

        smoothed_quats = []
        for i in range(n_frames):
            sw, sx, sy, sz = 0.0, 0.0, 0.0, 0.0
            for k_idx, k_val in enumerate(kernel):
                offset = k_idx - k_rad
                target_idx = i + offset
                if is_loop:
                    target_idx = target_idx % n_frames
                else:
                    target_idx = max(0, min(target_idx, n_frames - 1))
                
                q_target = quats[target_idx]
                sign = 1.0 if q_target.dot(quats[i]) >= 0 else -1.0
                sw += q_target.w * k_val * sign
                sx += q_target.x * k_val * sign
                sy += q_target.y * k_val * sign
                sz += q_target.z * k_val * sign
            
            sq = Quaternion((sw, sx, sy, sz))
            sq.normalize()
            smoothed_quats.append(sq)

        if is_loop:
            q_avg = smoothed_quats[0].slerp(smoothed_quats[-1], 0.5)
            smoothed_quats[0] = q_avg
            smoothed_quats[-1] = q_avg

        for i, f in enumerate(range(f0, f1 + 1)):
            sq = smoothed_quats[i]
            channels[0].keyframe_points[i].co[1] = sq.w
            channels[1].keyframe_points[i].co[1] = sq.x
            channels[2].keyframe_points[i].co[1] = sq.y
            channels[3].keyframe_points[i].co[1] = sq.z

    # Smooth Location curves (Hips)
    for bone_name, channels in loc_curves.items():
        for idx, fc in channels.items():
            if len(fc.keyframe_points) != n_frames:
                continue
            vals = [fc.evaluate(f) for f in range(f0, f1 + 1)]
            smoothed_vals = []
            for i in range(n_frames):
                s_val = 0.0
                for k_idx, k_val in enumerate(kernel):
                    offset = k_idx - k_rad
                    target_idx = i + offset
                    if is_loop:
                        target_idx = target_idx % n_frames
                    else:
                        target_idx = max(0, min(target_idx, n_frames - 1))
                    s_val += vals[target_idx] * k_val
                smoothed_vals.append(s_val)
            
            if is_loop:
                avg = (smoothed_vals[0] + smoothed_vals[-1]) / 2.0
                smoothed_vals[0] = avg
                smoothed_vals[-1] = avg

            for i, f in enumerate(range(f0, f1 + 1)):
                fc.keyframe_points[i].co[1] = smoothed_vals[i]

    # Convert all keyframes to BEZIER with AUTO_CLAMPED
    for fc in act.fcurves:
        for p in fc.keyframe_points:
            p.interpolation = 'BEZIER'
            p.handle_left_type = 'AUTO_CLAMPED'
            p.handle_right_type = 'AUTO_CLAMPED'
        fc.update()

# Universal feet pinning
mw = arm.matrix_world
sc = bpy.context.scene
hips_candidates = [b for b in arm.pose.bones if b.name.lower().endswith("hips")]
feet_candidates = [b for b in arm.pose.bones if any(x in b.name.lower() for x in ('foot', 'toe'))]

if hips_candidates and feet_candidates:
    hips = hips_candidates[0]
    feet = feet_candidates
    keep = arm.animation_data.action
    arm.animation_data.action = None
    arm.data.pose_position = 'REST'
    bpy.context.view_layer.update()
    rest = [(mw @ f.head).z for f in feet]
    arm.data.pose_position = 'POSE'
    
    lowest = lambda: min((mw @ b.head).z - r for b, r in zip(feet, rest))
    to_local = (mw.to_3x3() @ hips.bone.matrix_local.to_3x3()).inverted()
    
    for act_name in ACTIONS:
        act = bpy.data.actions.get(act_name)
        if not act: continue
        arm.animation_data.action = act
        curves = sorted((fc for fc in act.fcurves if fc.data_path == f'pose.bones["{hips.name}"].location'),
                        key=lambda fc: fc.array_index)
        if len(curves) == 3:
            f0 = int(act.frame_range[0])
            f1 = int(act.frame_range[1])
            lows = {}
            for f in range(f0, f1 + 1):
                sc.frame_set(f)
                bpy.context.view_layer.update()
                lows[f] = lowest()
            least = min(lows.values())
            is_run = (act_name == 'correr')
            shift = {f: to_local @ Vector((0, 0, -(least if is_run else lows[f]))) for f in range(f0, f1 + 1)}
            for fc in curves:
                vals = {f: fc.evaluate(f) + shift[f][fc.array_index] for f in range(f0, f1 + 1)}
                fc.keyframe_points.clear()
                for f, v in vals.items():
                    kp = fc.keyframe_points.insert(f, v)
                    kp.interpolation = 'BEZIER'
                    kp.handle_left_type = 'AUTO_CLAMPED'
                    kp.handle_right_type = 'AUTO_CLAMPED'
                fc.update()
            print(f"Feet pinned for '{act_name}'")
    arm.animation_data.action = keep

if OUT:
    bpy.ops.wm.save_as_mainfile(filepath=OUT)
    print("Salvo em:", OUT)
else:
    bpy.ops.wm.save_mainfile()
    print("Salvo no próprio arquivo.")
