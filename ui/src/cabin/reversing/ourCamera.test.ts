import { describe, expect, it } from 'vitest'
import { HEIGHT_TRUCK, MIRRORED, type OurCamera, PITCH, project } from './ourCamera'

// Facing back along +z from the origin, its right (the rig's) along -x.
const camera: OurCamera = { at: [0, 0], height: HEIGHT_TRUCK, back: [0, 1], right: [-1, 0] }

describe('our camera (rear_camera.rs) seen on the screen', () => {
  it('sees straight back in the middle, the far away near the middle height', () => {
    const near = project(camera, [0, 2], 800, 450)!
    const far = project(camera, [0, 40], 800, 450)!
    expect(near[0]).toBeCloseTo(400)
    expect(far[0]).toBeCloseTo(400)
    expect(near[1]).toBeGreaterThan(far[1])
    // Where it looks: PITCH down, so a point there is at the middle.
    const there = HEIGHT_TRUCK / Math.tan((PITCH * Math.PI) / 180)
    expect(project(camera, [0, there], 800, 450)![1]).toBeCloseTo(225)
  })

  it("shows the rig's right on the screen's right, as a reversing camera does", () => {
    const right = project(camera, [-1, 3], 800, 450)!
    expect(right[0] > 400).toBe(MIRRORED)
  })

  it('leaves out what is behind it', () => {
    expect(project(camera, [0, -3], 800, 450)).toBeNull()
  })
})
