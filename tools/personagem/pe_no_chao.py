# WestPlay: in-place jumps the game's physics lifts (docs/personagem/INCLUIR_PERSONAGEM.md, "Pulos").
# Each frame the hips are moved up or down so the lowest foot is on the floor: the feet never go through it nor
# float, and the body rises only with the game's capsule (a jump's own rise, capped at standing height as the
# first jumps were, left the feet 0.85 m up at the top, "climbing", and the running jump's landing leg 13 cm
# through the floor). On its own, on a combined scene (saving it):
#   blender -b <x_animada.blend> --python pe_no_chao.py -- pular,pular_correndo
import bpy, sys
from mathutils import Vector

FEET = ("LeftFoot", "LeftToeBase", "LeftToe_End", "RightFoot", "RightToeBase", "RightToe_End")


def pin(arm, names, constant=()):
    """Each frame's lowest foot on the floor; for the names in `constant` (a run: both feet off the ground
    between steps), one shift for the whole animation, its lowest point on the floor (the bounce kept)."""
    sc = bpy.context.scene; mw = arm.matrix_world
    bone = lambda n: [b for b in arm.pose.bones if b.name.endswith(n)][0]
    feet = [bone(n) for n in FEET]; hips = bone("Hips")
    keep = arm.animation_data.action
    arm.animation_data.action = None; arm.data.pose_position = 'REST'; bpy.context.view_layer.update()
    rest = [(mw @ f.head).z for f in feet]
    arm.data.pose_position = 'POSE'
    lowest = lambda: min((mw @ b.head).z - r for b, r in zip(feet, rest))
    # a world-up shift as the hips' own location (a root bone: its location is in its rest space)
    to_local = (mw.to_3x3() @ hips.bone.matrix_local.to_3x3()).inverted()
    for name in names:
        act = bpy.data.actions[name]; arm.animation_data.action = act
        curves = sorted((fc for fc in act.fcurves if fc.data_path == 'pose.bones["%s"].location' % hips.name),
                        key=lambda fc: fc.array_index)
        assert len(curves) == 3, name
        f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
        lows = {}; air = []
        for f in range(f0, f1 + 1):
            sc.frame_set(f); bpy.context.view_layer.update()
            lows[f] = lowest()
            if lows[f] > 0.03: air.append(f)
        least = min(lows.values())
        shift = {f: to_local @ Vector((0, 0, -(least if name in constant else low))) for f, low in lows.items()}
        # where the feet are off the ground (before pinning): the profile's takeoff and land (person.rs PEOPLE)
        runs = []
        for f in air:
            if runs and f == runs[-1][1] + 1: runs[-1][1] = f
            else: runs.append([f, f])
        longest = max(runs, key=lambda r: r[1] - r[0]) if runs else None
        print("AIR %s: feet off the ground in frames %s; the jump: takeoff frame(%d), land frame(%d) (%.2f s up; "
              "the game's is about 0.6 s)" % (name, runs, longest[0] - 1, longest[1] + 1, (longest[1] - longest[0] + 2) / 30)
              if longest else "AIR %s: the feet never leave the ground" % name)
        for fc in curves:
            vals = {f: fc.evaluate(f) + shift[f][fc.array_index] for f in range(f0, f1 + 1)}
            fc.keyframe_points.clear()
            for f, v in vals.items():
                fc.keyframe_points.insert(f, v).interpolation = 'LINEAR'
        lows = []
        for f in range(f0, f1 + 1):
            sc.frame_set(f); bpy.context.view_layer.update(); lows.append(lowest())
        print("PINNED %s: hips moved up to %.3f m; lowest foot now %.3f..%.3f"
              % (name, max(v.length for v in shift.values()), min(lows), max(lows)))
    arm.animation_data.action = keep


if __name__ == "__main__" and "--" in sys.argv:
    arm = [o for o in bpy.data.objects if o.type == 'ARMATURE'][0]
    pin(arm, sys.argv[sys.argv.index('--') + 1].split(','))
    bpy.context.scene.frame_set(1)
    bpy.ops.wm.save_mainfile()
