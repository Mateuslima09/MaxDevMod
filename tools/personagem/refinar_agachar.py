# WestPlay: refina e suaviza o conjunto completo de agachamento (agachar, agachado,
# andar_agachado, agachado_andar e agachado_parar), eliminando travamentos lineares
# e saltos bruscos de pose ("troca agressiva") entre todas as transições.
#
#   blender -b <cena.blend> --python refinar_agachar.py -- [radius=2]
#
import bpy, math, sys
from mathutils import Vector, Quaternion

A = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
RADIUS = int(A[0]) if len(A) > 0 and A[0] else 2

arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
sc = bpy.context.scene
mw = arm.matrix_world

if RADIUS == 1:
    weights = [1.0, 2.0, 1.0]
elif RADIUS == 2:
    weights = [1.0, 4.0, 6.0, 4.0, 1.0]
elif RADIUS == 3:
    weights = [1.0, 6.0, 15.0, 20.0, 15.0, 6.0, 1.0]
else:
    weights = [1.0]

total_w = sum(weights)
KERNEL = [w / total_w for w in weights]
K_RAD = len(KERNEL) // 2

# Smootherstep (C2 continuous: zero velocity and zero acceleration at endpoints)
smootherstep = lambda x: x * x * x * (x * (x * 6.0 - 15.0) + 10.0)


def curves_map(action):
    by = {}
    for fc in action.fcurves:
        by.setdefault(fc.data_path, {})[fc.array_index] = fc
    return by


def sample(by, path, size, frame):
    got = by.get(path, {})
    rest = (1.0, 0.0, 0.0, 0.0) if size == 4 else (1.0, 1.0, 1.0) if path.endswith("scale") else (0.0, 0.0, 0.0)
    return [got[i].evaluate(frame) if i in got else rest[i] for i in range(size)]


def mix_val(a, b, w, size):
    if size == 4:
        qa, qb = Quaternion(a), Quaternion(b)
        qa.normalize()
        qb.normalize()
        if qa.dot(qb) < 0:
            qb.negate()
        q = qa.slerp(qb, w)
        q.normalize()
        return [q.w, q.x, q.y, q.z]
    return [x + (y - x) * w for x, y in zip(a, b)]


def set_bezier(act):
    for fc in act.fcurves:
        for p in fc.keyframe_points:
            p.interpolation = 'BEZIER'
            p.handle_left_type = 'AUTO_CLAMPED'
            p.handle_right_type = 'AUTO_CLAMPED'
        fc.update()


def smooth_action_inplace(act, is_loop=False):
    f0 = int(act.frame_range[0])
    f1 = int(act.frame_range[1])
    n_frames = f1 - f0 + 1
    quat_curves = {}
    loc_curves = {}

    for fc in act.fcurves:
        if 'rotation_quaternion' in fc.data_path:
            bone_name = fc.data_path.split('"')[1]
            quat_curves.setdefault(bone_name, {})[fc.array_index] = fc
        elif 'location' in fc.data_path:
            bone_name = fc.data_path.split('"')[1]
            loc_curves.setdefault(bone_name, {})[fc.array_index] = fc

    for bone_name, channels in quat_curves.items():
        if len(channels) < 4 or len(channels[0].keyframe_points) != n_frames:
            continue
        quats = [Quaternion((channels[0].evaluate(f), channels[1].evaluate(f),
                             channels[2].evaluate(f), channels[3].evaluate(f)))
                 for f in range(f0, f1 + 1)]
        for i in range(1, len(quats)):
            if quats[i].dot(quats[i - 1]) < 0:
                quats[i].negate()

        smoothed = []
        for i in range(n_frames):
            sw = sx = sy = sz = 0.0
            for k_idx, k_val in enumerate(KERNEL):
                t = i + (k_idx - K_RAD)
                t = (t % n_frames) if is_loop else max(0, min(t, n_frames - 1))
                qt = quats[t]
                sign = 1.0 if qt.dot(quats[i]) >= 0 else -1.0
                sw += qt.w * k_val * sign
                sx += qt.x * k_val * sign
                sy += qt.y * k_val * sign
                sz += qt.z * k_val * sign
            sq = Quaternion((sw, sx, sy, sz))
            sq.normalize()
            smoothed.append(sq)

        if is_loop:
            q_avg = smoothed[0].slerp(smoothed[-1], 0.5)
            q_avg.normalize()
            smoothed[0] = q_avg
            smoothed[-1] = q_avg

        for i in range(n_frames):
            sq = smoothed[i]
            channels[0].keyframe_points[i].co[1] = sq.w
            channels[1].keyframe_points[i].co[1] = sq.x
            channels[2].keyframe_points[i].co[1] = sq.y
            channels[3].keyframe_points[i].co[1] = sq.z

    for bone_name, channels in loc_curves.items():
        for idx, fc in channels.items():
            if len(fc.keyframe_points) != n_frames:
                continue
            vals = [fc.evaluate(f) for f in range(f0, f1 + 1)]
            smoothed = []
            for i in range(n_frames):
                sv = 0.0
                for k_idx, k_val in enumerate(KERNEL):
                    t = i + (k_idx - K_RAD)
                    t = (t % n_frames) if is_loop else max(0, min(t, n_frames - 1))
                    sv += vals[t] * k_val
                smoothed.append(sv)
            if is_loop:
                avg = 0.5 * (smoothed[0] + smoothed[-1])
                smoothed[0] = avg
                smoothed[-1] = avg
            for i in range(n_frames):
                fc.keyframe_points[i].co[1] = smoothed[i]

    set_bezier(act)


def pin_feet(action_names):
    hips_list = [b for b in arm.pose.bones if b.name.lower().endswith("hips")]
    feet = [b for b in arm.pose.bones if any(x in b.name.lower() for x in ('foot', 'toe'))]
    if not hips_list or not feet:
        return
    hips = hips_list[0]
    keep = arm.animation_data.action
    arm.animation_data.action = None
    arm.data.pose_position = 'REST'
    bpy.context.view_layer.update()
    rest = [(mw @ f.head).z for f in feet]
    arm.data.pose_position = 'POSE'
    lowest = lambda: min((mw @ b.head).z - r for b, r in zip(feet, rest))
    to_local = (mw.to_3x3() @ hips.bone.matrix_local.to_3x3()).inverted()

    for name in action_names:
        act = bpy.data.actions.get(name)
        if not act:
            continue
        arm.animation_data.action = act
        curves = sorted((fc for fc in act.fcurves if fc.data_path == f'pose.bones["{hips.name}"].location'),
                        key=lambda fc: fc.array_index)
        if len(curves) != 3:
            continue
        f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
        lows = {}
        for f in range(f0, f1 + 1):
            sc.frame_set(f)
            bpy.context.view_layer.update()
            lows[f] = lowest()
        shift = {f: to_local @ Vector((0, 0, -lows[f])) for f in range(f0, f1 + 1)}
        for fc in curves:
            vals = {f: fc.evaluate(f) + shift[f][fc.array_index] for f in range(f0, f1 + 1)}
            fc.keyframe_points.clear()
            for f, v in vals.items():
                kp = fc.keyframe_points.insert(f, v)
                kp.interpolation = 'BEZIER'
                kp.handle_left_type = 'AUTO_CLAMPED'
                kp.handle_right_type = 'AUTO_CLAMPED'
            fc.update()
        print(f"PINNED '{name}': lowest foot grounded")
    arm.animation_data.action = keep


def rebuild_action(name, paths, frames_fn):
    # Pre-evaluate all data BEFORE removing any existing action (avoids freed FCurve pointers)
    precomputed = []
    for path in paths:
        size = 4 if path.endswith("rotation_quaternion") else 3
        precomputed.append((path, size, frames_fn(path, size)))
    if name in bpy.data.actions:
        bpy.data.actions.remove(bpy.data.actions[name])
    act = bpy.data.actions.new(name)
    act.use_fake_user = True
    for path, size, vals in precomputed:
        group = path.split('"')[1] if '"' in path else None
        made = [act.fcurves.new(path, index=i, action_group=group) for i in range(size)]
        for f, v in enumerate(vals, start=1):
            for fc, x in zip(made, v):
                kp = fc.keyframe_points.insert(f, x)
                kp.interpolation = 'BEZIER'
                kp.handle_left_type = 'AUTO_CLAMPED'
                kp.handle_right_type = 'AUTO_CLAMPED'
        for fc in made:
            fc.update()
    return act


# 1. Smooth andar_agachado (32 frames loop)
act_cwalk = bpy.data.actions["andar_agachado"]
smooth_action_inplace(act_cwalk, is_loop=True)
pin_feet(["andar_agachado"])
print("1. andar_agachado suavizado e fechado em loop.")

# 2. Smooth agachar (40 frames) and seamlessly blend ends into parado(1)
act_crouch = bpy.data.actions["agachar"]
act_idle = bpy.data.actions["parado"]
smooth_action_inplace(act_crouch, is_loop=False)

all_paths = sorted({fc.data_path for act in (act_crouch, act_cwalk, act_idle) for fc in act.fcurves})
crouch_by = curves_map(act_crouch)
idle_by = curves_map(act_idle)
cwalk_by = curves_map(act_cwalk)

i0 = int(act_idle.frame_range[0])
c0, c1 = int(act_crouch.frame_range[0]), int(act_crouch.frame_range[1])
EASE_LEN = 8  # smooth 8-frame S-curve blend into/out of standing

def build_agachar(path, size):
    stand = sample(idle_by, path, size, i0)
    out = []
    for f in range(c0, c1 + 1):
        v = sample(crouch_by, path, size, f)
        k_start = f - c0
        k_end = c1 - f
        if k_start < EASE_LEN:
            w = smootherstep(k_start / EASE_LEN)
            v = mix_val(stand, v, w, size)
        elif k_end < EASE_LEN:
            w = smootherstep(k_end / EASE_LEN)
            v = mix_val(stand, v, w, size)
        out.append(v)
    return out

act_crouch = rebuild_action("agachar", all_paths, build_agachar)
pin_feet(["agachar"])
print("2. agachar suavizado e costurado exatamente em parado(1).")

# Re-read agachar after foot pinning so bottom pose (frame 20) is exact!
crouch_by = curves_map(act_crouch)
BOTTOM_FRAME = 20

# 3. Rebuild agachado (2 frames, exact match to agachar frame 20)
rebuild_action("agachado", all_paths, lambda p, s: [sample(crouch_by, p, s, BOTTOM_FRAME)] * 2)
print("3. agachado sincronizado exatamente com agachar(20).")

# 4. Rebuild agachado_andar (10 frames: agachar(20) -> andar_agachado(10))
N_START = 10
w0, w1 = int(act_cwalk.frame_range[0]), int(act_cwalk.frame_range[1])

def build_cstart(path, size):
    k = sample(crouch_by, path, size, BOTTOM_FRAME)
    out = []
    for i in range(N_START):
        w = smootherstep(i / (N_START - 1))
        target = sample(cwalk_by, path, size, w0 + i)
        out.append(mix_val(k, target, w, size))
    return out

rebuild_action("agachado_andar", all_paths, build_cstart)
pin_feet(["agachado_andar"])
print("4. agachado_andar reconstruído com smootherstep e zero salto.")

# 5. Rebuild agachado_parar (8 pieces x 10 frames = 80 frames: andar_agachado -> agachar(20))
PIECES = 8
N_SETTLE = 10
cycle = w1 - w0

def build_csettle(path, size):
    b = sample(crouch_by, path, size, BOTTOM_FRAME)
    out = []
    for piece in range(PIECES):
        a = sample(cwalk_by, path, size, w0 + piece * cycle / PIECES)
        for i in range(N_SETTLE):
            w = smootherstep(i / (N_SETTLE - 1))
            out.append(mix_val(a, b, w, size))
    return out

rebuild_action("agachado_parar", all_paths, build_csettle)
pin_feet(["agachado_parar"])
print("5. agachado_parar (8 peças) reconstruído com smootherstep e zero salto.")

arm.animation_data.action = bpy.data.actions.get("andar")
sc.frame_set(1)
bpy.ops.wm.save_mainfile()
print("CONCLUÍDO e salvo com sucesso!")
