export const METRIC_NAMES = Object.freeze({
  successfulRun: 'successful_run',
  readOnly: 'read_only',
  noFailedTools: 'no_failed_tools',
  directRustcodegraphCall: 'direct_rustcodegraph_call',
  successfulRustcodegraphCall: 'rustcodegraph_call_success',
  rustcodegraphOperation: 'rustcodegraph_operation',
  noNavigationFallback: 'no_navigation_fallback',
  expectedFacts: 'expected_facts',
  toolCallBudget: 'tool_call_budget',
});

const WEIGHTS = Object.freeze({
  [METRIC_NAMES.successfulRun]: 10,
  [METRIC_NAMES.readOnly]: 10,
  [METRIC_NAMES.noFailedTools]: 10,
  [METRIC_NAMES.directRustcodegraphCall]: 20,
  [METRIC_NAMES.successfulRustcodegraphCall]: 15,
  [METRIC_NAMES.rustcodegraphOperation]: 10,
  [METRIC_NAMES.noNavigationFallback]: 10,
  [METRIC_NAMES.expectedFacts]: 10,
  [METRIC_NAMES.toolCallBudget]: 5,
});

const HARD_GATES = Object.freeze([
  METRIC_NAMES.successfulRun,
  METRIC_NAMES.readOnly,
  METRIC_NAMES.noFailedTools,
  METRIC_NAMES.directRustcodegraphCall,
  METRIC_NAMES.successfulRustcodegraphCall,
]);

function normalizedScore(value) {
  const score = Number(value);
  if (!Number.isFinite(score)) {
    return 0;
  }
  return Math.min(1, Math.max(0, score));
}

export default function scoreTrajectory(namedScores, context = {}) {
  const scores = namedScores ?? {};
  const points = Object.entries(WEIGHTS).reduce(
    (total, [name, weight]) => total + normalizedScore(scores[name]) * weight,
    0,
  );
  const score = points / 100;
  const failedGates = HARD_GATES.filter((name) => normalizedScore(scores[name]) < 1);
  const threshold = Number(context?.threshold);
  const belowThreshold = Number.isFinite(threshold) && score < threshold;
  const pass = failedGates.length === 0 && !belowThreshold;
  const scoreLabel = Number.isInteger(points) ? String(points) : points.toFixed(2).replace(/0+$/, '');

  let detail = '全部硬门槛通过';
  if (failedGates.length > 0) {
    detail = `未通过硬门槛：${failedGates.join(', ')}`;
  } else if (belowThreshold) {
    detail = `低于阈值 ${threshold}`;
  }
  return {
    pass,
    score,
    reason: `总分 ${scoreLabel}/100；${detail}`,
  };
}
