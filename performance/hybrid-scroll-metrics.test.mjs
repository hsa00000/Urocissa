import assert from 'node:assert/strict'
import test from 'node:test'

import {
  analyzeHandoffFrames,
  classifyWheelDisplacement
} from './hybrid-scroll-metrics.mjs'

test('measures projection continuity independently from logical movement', () => {
  const result = analyzeHandoffFrames({
    anchorStart: '42',
    expectedDirection: 1,
    wheelEvents: [{ time: 5 }],
    frames: [
      {
        time: 0,
        anchorStart: '42',
        anchorTop: 200,
        logicalTop: 2_999,
        mode: 'native-top',
        generation: 1
      },
      {
        time: 16,
        sampledAt: 17,
        anchorStart: '42',
        anchorTop: 199,
        logicalTop: 3_000,
        mode: 'compensated',
        generation: 2
      }
    ]
  })

  assert.equal(result.handoffProjectionResidualPx, 0)
  assert.equal(result.handoffFrameGapMs, 16)
  assert.equal(result.inputToFirstVisualMotionMs, 12)
  assert.equal(result.modeTransitionFrameCount, 1)
  assert.equal(result.generationTransitionFrameCount, 1)
})

test('detects a projection jump even when movement stays in the requested direction', () => {
  const result = analyzeHandoffFrames({
    anchorStart: '42',
    expectedDirection: 1,
    wheelEvents: [{ time: 5 }],
    frames: [
      {
        time: 0,
        anchorStart: '42',
        anchorTop: 200,
        logicalTop: 2_999,
        mode: 'native-top',
        generation: 1
      },
      {
        time: 16,
        anchorStart: '42',
        anchorTop: 195,
        logicalTop: 3_000,
        mode: 'compensated',
        generation: 2
      }
    ]
  })

  assert.equal(result.handoffProjectionResidualPx, 4)
})

test('reports missing visual responsiveness instead of manufacturing a zero', () => {
  const result = analyzeHandoffFrames({
    anchorStart: '42',
    expectedDirection: 1,
    wheelEvents: [{ time: 5 }],
    frames: [
      {
        time: 0,
        anchorStart: '42',
        anchorTop: 200,
        logicalTop: 2_999,
        mode: 'native-top',
        generation: 1
      },
      {
        time: 16,
        anchorStart: '42',
        anchorTop: 200,
        logicalTop: 2_999,
        mode: 'native-top',
        generation: 1
      }
    ]
  })

  assert.equal(result.inputToFirstVisualMotionMs, null)
  assert.equal(result.handoffProjectionResidualPx, null)
})

test('classifies a truncated transition pulse without calling it discontinuous', () => {
  const result = classifyWheelDisplacement({
    actualDisplacementPx: 1,
    expectedDisplacementPx: 100,
    modeChanged: true,
    tolerancePx: 1
  })

  assert.equal(result.movementStayedForward, true)
  assert.equal(result.wheelDisplacementErrorPx, 99)
  assert.equal(result.wheelDisplacementRatio, 0.01)
  assert.equal(result.truncatedHandoffPulse, true)
})

test('does not allow truncation to hide on an ordinary control pulse', () => {
  const result = classifyWheelDisplacement({
    actualDisplacementPx: 1,
    expectedDisplacementPx: 100,
    modeChanged: false,
    tolerancePx: 1
  })

  assert.equal(result.truncatedHandoffPulse, false)
  assert.equal(result.wheelDisplacementErrorPx, 99)
})
