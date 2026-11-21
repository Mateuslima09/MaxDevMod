# WestPlay: do the forearms and hands go into the body, in each animation? (the stout man's arms went through his
# belly, owner 2026-11-14). For every few frames, each forearm/hand vertex is checked against the torso (the faces
# that move with the spine and hips): how deep it is behind the nearest torso face. A touch is fine (hands resting
# on the back); deeper than LIMIT is a problem.
#   blender -b <x_animada.blend> --python conferir_bracos.py -- [step] [action,action,...] [limit cm]
# Prints, per action, the deepest point (cm), at which frame and which side, and how many frames go past LIMIT.
import bpy, sys
from mathutils.bvhtree import BVHTree
A = sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else []
STEP = int(A[0]) if A and A[0] else 2
arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
names = A[1].split(",") if len(A) > 1 and A[1] else sorted(a.name for a in bpy.data.actions if a.frame_range[1] > 2)
LIMIT = (float(A[2]) if len(A) > 2 else 1.5) / 100
body = max((o for o in bpy.data.objects if o.type == 'MESH' and o.find_armature() == arm), key=lambda o: len(o.data.vertices))
groups = {g.index: g.name.split(":")[-1] for g in body.vertex_groups}
def owner(v):
    best = max(v.groups, key=lambda g: g.weight, default=None)
    return groups.get(best.group, "") if best else ""
TORSO = {"Hips", "Spine", "Spine1", "Spine2"}
torso_v = set(); arm_v = {}
for v in body.data.vertices:
    o = owner(v)
    if o in TORSO: torso_v.add(v.index)
    elif o.startswith(("LeftForeArm", "LeftHand")): arm_v[v.index] = "esquerdo"
    elif o.startswith(("RightForeArm", "RightHand")): arm_v[v.index] = "direito"
torso_f = [p.index for p in body.data.polygons if all(i in torso_v for i in p.vertices)]
print("BODY %s: %d torso faces, %d forearm/hand vertices" % (body.name, len(torso_f), len(arm_v)))
deps = bpy.context.evaluated_depsgraph_get()
bad_total = 0
for name in names:
    act = bpy.data.actions[name]; arm.animation_data.action = act
    f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
    worst = (0.0, f0, ""); bad = 0
    for f in list(range(f0, f1 + 1, STEP)) + [f1]:
        bpy.context.scene.frame_set(f); deps.update()
        me = body.evaluated_get(deps).to_mesh()
        co = [body.matrix_world @ v.co for v in me.vertices]
        tree = BVHTree.FromPolygons([co[i] for i in range(len(co))], [tuple(me.polygons[i].vertices) for i in torso_f])
        deep = (0.0, "")
        for i, side in arm_v.items():
            hit = tree.find_nearest(co[i], 0.25)
            if hit[0] is None: continue
            d = (co[i] - hit[0]).dot(hit[1])
            if d < -deep[0]: deep = (-d, side)
        body.evaluated_get(deps).to_mesh_clear()
        if deep[0] > LIMIT: bad += 1
        if deep[0] > worst[0]: worst = (deep[0], f, deep[1])
    bad_total += bad
    print("BRACO %-16s mais fundo %4.1f cm (quadro %d, %s)  quadros acima de %.1f cm: %d" % (name, worst[0] * 100, worst[1], worst[2] or "-", LIMIT * 100, bad))
print("RESULTADO", "ok" if bad_total == 0 else "PROBLEMA em %d quadros" % bad_total)
