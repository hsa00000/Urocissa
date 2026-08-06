export const HANDOFF_MOTION_THRESHOLD_PX = 0.5

function isFiniteFrame(frame) {
  return (
    frame !== null &&
    typeof frame === 'object' &&
    Number.isFinite(frame.time) &&
    Number.isFinite(frame.anchorTop) &&
    Number.isFinite(frame.logicalTop)
  )
}

function observedAt(frame) {
  return Number.isFinite(frame.sampledAt) ? frame.sampledAt : frame.time
}

export function analyzeHandoffFrames({
  frames,
  wheelEvents,
  anchorStart,
  expectedDirection,
  motionThresholdPx = HANDOFF_MOTION_THRESHOLD_PX
}) {
  const trackedFrames = frames.filter(
    (frame) =>
      isFiniteFrame(frame) && String(frame.anchorStart) === String(anchorStart)
  )
  const transitions = []

  for (let index = 1; index < trackedFrames.length; index += 1) {
    const previous = trackedFrames[index - 1]
    const current = trackedFrames[index]
    const modeChanged = previous.mode !== current.mode
    const generationChanged = previous.generation !== current.generation
    if (!modeChanged && !generationChanged) continue

    transitions.push({
      startTime: previous.time,
      endTime: current.time,
      frameGapMs: current.time - previous.time,
      fromMode: previous.mode ?? null,
      toMode: current.mode ?? null,
      fromGeneration: previous.generation ?? null,
      toGeneration: current.generation ?? null,
      anchorDeltaPx: current.anchorTop - previous.anchorTop,
      logicalDeltaPx: current.logicalTop - previous.logicalTop,
      projectionResidualPx: Math.abs(
        current.anchorTop - previous.anchorTop + current.logicalTop - previous.logicalTop
      ),
      modeChanged,
      generationChanged
    })
  }

  const firstWheel = wheelEvents[0] ?? null
  const baselineFrame =
    firstWheel === null
      ? null
      : trackedFrames.findLast((frame) => observedAt(frame) <= firstWheel.time) ?? null
  const firstMotionFrame =
    firstWheel === null || baselineFrame === null || expectedDirection === 0
      ? null
      : trackedFrames.find(
          (frame) =>
            observedAt(frame) > firstWheel.time &&
            (baselineFrame.anchorTop - frame.anchorTop) * expectedDirection >=
              motionThresholdPx
        ) ?? null

  return {
    trackedFrameCount: trackedFrames.length,
    transitionFrameCount: transitions.length,
    modeTransitionFrameCount: transitions.filter((transition) => transition.modeChanged)
      .length,
    generationTransitionFrameCount: transitions.filter(
      (transition) => transition.generationChanged
    ).length,
    handoffProjectionResidualPx:
      transitions.length === 0
        ? null
        : Math.max(...transitions.map((transition) => transition.projectionResidualPx)),
    handoffFrameGapMs:
      transitions.length === 0
        ? null
        : Math.max(...transitions.map((transition) => transition.frameGapMs)),
    inputToFirstVisualMotionMs:
      firstWheel === null || firstMotionFrame === null
        ? null
        : observedAt(firstMotionFrame) - firstWheel.time,
    transitions
  }
}

export function classifyWheelDisplacement({
  actualDisplacementPx,
  expectedDisplacementPx,
  modeChanged,
  tolerancePx
}) {
  const expectedMagnitude = Math.abs(expectedDisplacementPx)
  const actualMagnitude = Math.abs(actualDisplacementPx)
  const expectedDirection = Math.sign(expectedDisplacementPx)
  const movementStayedForward =
    actualDisplacementPx * expectedDirection >= -tolerancePx &&
    actualMagnitude <= expectedMagnitude + tolerancePx
  const wheelDisplacementErrorPx = Math.abs(
    actualDisplacementPx - expectedDisplacementPx
  )

  return {
    movementStayedForward,
    wheelDisplacementErrorPx,
    wheelDisplacementRatio:
      expectedMagnitude === 0 ? 1 : actualMagnitude / expectedMagnitude,
    truncatedHandoffPulse:
      modeChanged &&
      movementStayedForward &&
      actualMagnitude < expectedMagnitude - tolerancePx
  }
}
