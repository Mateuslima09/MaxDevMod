# WestPlay: the arms kept out of the body in every animation, for a person much wider than the one the animations
# came from (reaproveitar_animacoes.py). The stout man (wg) on the tattooed man's animations had his hands and
# forearms up to 18 cm inside his belly and back (owner 2026-11-14; measured with conferir_bracos.py).
#
# For every frame and each side, the upper arm is turned at the shoulder, outward (about the forward axis)
# and backward or forward (about the side axis), by the smallest turn that leaves the forearm and hand no deeper
# than LIMIT behind the torso's surface (the faces that move with the spine and hips). The forearm and hand go with
# the upper arm as they were. The turns are then widened and smoothed over time (looping actions around their
# ends), so the arm does not jitter, and keyed on every frame. The pelvis and legs are not touched.
#   blender -b <x_animada.blend> --python bracos_abertos.py -- [action,action,...] [saida.blend] [limit cm]
# Then conferir_bracos.py on the result.
import bpy, math, sys
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree
A = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
names = A[0].split(",") if A and A[0] else sorted(a.name for a in bpy.data.actions if a.frame_range[1] > 2)
OUT = A[1] if len(A) > 1 and A[1] else None
LIMIT = (float(A[2]) if len(A) > 2 else 1.0) / 100
STEP = 1
LOOPS = {"andar", "correr", "parado", "andar_agachado", "celular", "celular_andando"}
OUTS = range(0, 44, 4)                       # degrees outward (more looks like a crab: owner sheet 2026-11-14)
BACKS = (0, 10, -10, 20, -20, 30, -30)       # backward; more lifts a swung-back arm up behind    # degrees backward (negative: forward)
body = max((o for o in bpy.data.objects if o.type == 'MESH' and o.find_armature() == arm), key=lambda o: len(o.data.vertices))
groups = {g.index: g.name.split(":")[-1] for g in body.vertex_groups}
def owner(v):
    best = max(v.groups, key=lambda g: g.weight, default=None)
    return groups.get(best.group, "") if best else ""
TORSO = {"Hips", "Spine", "Spine1", "Spine2"}
torso_v = {v.index for v in body.data.vertices if owner(v) in TORSO}
torso_f = [tuple(p.vertices) for p in body.data.polygons if all(i in torso_v for i in p.vertices)]
side_v = {"Left": [], "Right": []}
for v in body.data.vertices:
    o = owner(v)
    for s in side_v:
        if o.startswith((s + "ForeArm", s + "Hand")): side_v[s].append(v.index)
bone = lambda s: [b for b in arm.pose.bones if b.name.endswith(s)][0]
SIGN = {"Left": -1, "Right": 1}
UP = {s: bone(s + "Arm") for s in SIGN}
W = arm.matrix_world.to_3x3()
AXIS = (W.inverted() @ Vector((0, 1, 0))).normalized()   # the world's forward axis, in the armature's space
SIDE = (W.inverted() @ Vector((1, 0, 0))).normalized()
def turn_world(s, out, back):
    return Matrix.Rotation(math.radians(out) * SIGN[s], 3, Vector((0, 1, 0))) @ Matrix.Rotation(math.radians(back), 3, Vector((1, 0, 0)))
def depth(tree, pts, h, R):
    deep = 0.0
    for p in pts:
        q = h + R @ (p - h)
        hit = tree.find_nearest(q, 0.3)
        if hit[0] is not None:
            d = -(q - hit[0]).dot(hit[1])
            if d > deep: deep = d
    return deep
CANDIDATES = [(o, b) for o in OUTS for b in BACKS]
# The smallest turn, and close to the last frame's: where the arm is right at the side, backward and forward
# can both work, and picking one then the other would cancel out when smoothed.
def ranked(prev):
    return sorted(CANDIDATES, key=lambda c: c[0] + abs(c[1]) * 3 + abs(c[0] - prev[0]) + 2 * abs(c[1] - prev[1]))
deps = bpy.context.evaluated_depsgraph_get()
def solve(name):
    act = bpy.data.actions[name]; arm.animation_data.action = act
    f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
    frames = list(range(f0, f1 + 1, STEP)) + ([f1] if (f1 - f0) % STEP else [])
    found = {s: [] for s in SIGN}
    for f in frames:
        bpy.context.scene.frame_set(f); deps.update()
        me = body.evaluated_get(deps).to_mesh()
        co = [body.matrix_world @ v.co for v in me.vertices]
        body.evaluated_get(deps).to_mesh_clear()
        tree = BVHTree.FromPolygons(co, torso_f)
        for s in SIGN:
            h = arm.matrix_world @ UP[s].head
            pts = [co[i] for i in side_v[s]]
            best = None
            prev = found[s][-1] if found[s] else (0, 0)
            for o, b in ranked(prev):
                d = depth(tree, pts, h, turn_world(s, o, b))
                if d <= LIMIT: best = (o, b); break
                if best is None or d < best[2]: best = (o, b, d)
            found[s].append(best[:2])
    return frames, found
def smooth(values, loop, widen=3, blur=2):
    n = len(values); at = (lambda i: values[i % n]) if loop else (lambda i: values[min(max(i, 0), n - 1)])
    # widen: keep the arm out a little before and after it is needed; then a moving average
    wide = [max((at(i + k) for k in range(-widen, widen + 1)), key=abs) for i in range(n)]
    at2 = (lambda i: wide[i % n]) if loop else (lambda i: wide[min(max(i, 0), n - 1)])
    return [sum(at2(i + k) for k in range(-blur, blur + 1)) / (2 * blur + 1) for i in range(n)]
def bake(name):
    # every frame keyed first: some actions (abastecer) have only their two ends keyed, and a key set on one frame
    # would change the frames after it before they are turned
    act = bpy.data.actions[name]; arm.animation_data.action = act
    for f in range(int(act.frame_range[0]), int(act.frame_range[1]) + 1):
        bpy.context.scene.frame_set(f)
        for up in UP.values(): up.keyframe_insert("rotation_quaternion", frame=f, group=up.name)
TOTAL = {}
def fix(name):
    frames, found = solve(name)
    if all(c == (0, 0) for s in SIGN for c in found[s]): return False
    act = bpy.data.actions[name]; loop = name in LOOPS
    curves = {}
    for s in SIGN:
        outs = smooth([o for o, _ in found[s]], loop)
        backs = smooth([b for _, b in found[s]], loop)
        curves[s] = (outs, backs)
    f0, f1 = frames[0], frames[-1]
    def at(seq, f):
        for i in range(len(frames) - 1):
            if frames[i] <= f <= frames[i + 1]:
                t = (f - frames[i]) / (frames[i + 1] - frames[i]); return seq[i] * (1 - t) + seq[i + 1] * t
        return seq[-1]
    if loop:   # a looping action ends where it starts
        for s in SIGN:
            for seq in curves[s]:
                seq[0] = seq[-1] = max(seq[0], seq[-1], key=abs)
    # all passes together stay within the turns allowed
    acc = TOTAL.setdefault(name, {s: [[0.0, 0.0] for _ in frames] for s in SIGN})
    for s in SIGN:
        for k, (lo, hi) in enumerate(((0, max(OUTS)), (min(BACKS), max(BACKS)))):
            seq = curves[s][k]
            for i in range(len(seq)):
                seq[i] = min(max(acc[s][i][k] + seq[i], lo), hi) - acc[s][i][k]
                acc[s][i][k] += seq[i]
    for f in range(f0, f1 + 1):
        bpy.context.scene.frame_set(f); bpy.context.view_layer.update()
        for s, up in UP.items():
            out, back = at(curves[s][0], f), at(curves[s][1], f)
            h = up.matrix.translation.copy()
            R = Matrix.Rotation(math.radians(out) * SIGN[s], 4, AXIS) @ Matrix.Rotation(math.radians(back), 4, SIDE)
            up.matrix = Matrix.Translation(h) @ R @ Matrix.Translation(-h) @ up.matrix; bpy.context.view_layer.update()
            up.keyframe_insert("rotation_quaternion", frame=f, group=up.name)
    for fc in act.fcurves:
        if any(fc.data_path.startswith('pose.bones["%s"]' % up.name) for up in UP.values()):
            for p in fc.keyframe_points: p.interpolation = 'LINEAR'
    print("ARMS %-16s left out max %4.1f back %5.1f..%5.1f | right out max %4.1f back %5.1f..%5.1f" % (
        name, max(curves["Left"][0]), min(curves["Left"][1]), max(curves["Left"][1]),
        max(curves["Right"][0]), min(curves["Right"][1]), max(curves["Right"][1])))
    return True
# A turn near the body's edge can leave a few frames just short: again on the result (three times at most; the
# turns of all passes together stay within OUTS and BACKS).
for name in names:
    bake(name)
    for _ in range(3):
        if not fix(name): break
arm.animation_data.action = bpy.data.actions.get("parado"); bpy.context.scene.frame_set(1)
if OUT: bpy.ops.wm.save_as_mainfile(filepath=OUT, copy=True)
else: bpy.ops.wm.save_mainfile()
