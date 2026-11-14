use super::*;
use pretty_assertions::assert_eq;

fn snapshot(queued_lines: usize, oldest_age_ms: u64) -> QueueSnapshot {
    QueueSnapshot { queued_lines, oldest_age: Some(Duration::from_millis(oldest_age_ms)) }
}

#[test]
// 空队列与轻压力默认使用一次一行的平滑排空。
fn smooth_mode_is_default() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let now = Instant::now();

    let decision = policy.decide(snapshot(/*queued_lines*/ 1, /*oldest_age_ms*/ 10), now);
    assert_eq!(decision.mode, ChunkingMode::Smooth);
    assert_eq!(decision.entered_catch_up, false);
    assert_eq!(decision.drain_plan, DrainPlan::Single);
}

#[test]
// 队列深度达到进入阈值即可切换追赶模式。
fn enters_catch_up_on_depth_threshold() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let now = Instant::now();

    let decision = policy.decide(snapshot(/*queued_lines*/ 8, /*oldest_age_ms*/ 10), now);
    assert_eq!(decision.mode, ChunkingMode::CatchUp);
    assert_eq!(decision.entered_catch_up, true);
    assert_eq!(decision.drain_plan, DrainPlan::Batch(8));
}

#[test]
// 即使深度较小，最老行超龄也应触发追赶。
fn enters_catch_up_on_age_threshold() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let now = Instant::now();

    let decision = policy.decide(snapshot(/*queued_lines*/ 2, /*oldest_age_ms*/ 120), now);
    assert_eq!(decision.mode, ChunkingMode::CatchUp);
    assert_eq!(decision.entered_catch_up, true);
    assert_eq!(decision.drain_plan, DrainPlan::Batch(2));
}

#[test]
// 严重积压使用更快批量计划，而非维持平滑节拍。
fn severe_backlog_uses_faster_paced_batches() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let now = Instant::now();
    let _ = policy.decide(snapshot(/*queued_lines*/ 9, /*oldest_age_ms*/ 10), now);

    let decision = policy.decide(
        snapshot(/*queued_lines*/ 64, /*oldest_age_ms*/ 10),
        now + Duration::from_millis(5),
    );
    assert_eq!(decision.mode, ChunkingMode::CatchUp);
    assert_eq!(decision.drain_plan, DrainPlan::Batch(64));
}

#[test]
// 追赶模式的批量计划应覆盖决定时可见的全部积压。
fn catch_up_batch_drains_current_backlog() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let now = Instant::now();
    let decision = policy.decide(snapshot(/*queued_lines*/ 512, /*oldest_age_ms*/ 400), now);
    assert_eq!(decision.mode, ChunkingMode::CatchUp);
    assert_eq!(decision.drain_plan, DrainPlan::Batch(512));
}

#[test]
// 低压力必须持续完整保持窗口后才能退出追赶。
fn exits_catch_up_after_hysteresis_hold() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let t0 = Instant::now();

    let _ = policy.decide(snapshot(/*queued_lines*/ 9, /*oldest_age_ms*/ 10), t0);
    assert_eq!(policy.mode(), ChunkingMode::CatchUp);

    let pre_hold = policy.decide(
        snapshot(/*queued_lines*/ 2, /*oldest_age_ms*/ 40),
        t0 + Duration::from_millis(200),
    );
    assert_eq!(pre_hold.mode, ChunkingMode::CatchUp);

    let post_hold = policy.decide(
        snapshot(/*queued_lines*/ 2, /*oldest_age_ms*/ 40),
        t0 + Duration::from_millis(460),
    );
    assert_eq!(post_hold.mode, ChunkingMode::Smooth);
    assert_eq!(post_hold.drain_plan, DrainPlan::Single);
}

#[test]
// 队列变空时策略立即恢复基线平滑状态。
fn drops_back_to_smooth_when_idle() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let now = Instant::now();
    let _ = policy.decide(snapshot(/*queued_lines*/ 9, /*oldest_age_ms*/ 10), now);
    assert_eq!(policy.mode(), ChunkingMode::CatchUp);

    let decision = policy.decide(
        QueueSnapshot { queued_lines: 0, oldest_age: None },
        now + Duration::from_millis(20),
    );
    assert_eq!(decision.mode, ChunkingMode::Smooth);
    assert_eq!(decision.drain_plan, DrainPlan::Single);
}

#[test]
// 刚退出追赶后的普通压力不会在冷却窗口内重新进入。
fn holds_reentry_after_catch_up_exit() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let t0 = Instant::now();

    let entered = policy.decide(snapshot(/*queued_lines*/ 8, /*oldest_age_ms*/ 20), t0);
    assert_eq!(entered.mode, ChunkingMode::CatchUp);

    let drained = policy.decide(
        QueueSnapshot { queued_lines: 0, oldest_age: None },
        t0 + Duration::from_millis(20),
    );
    assert_eq!(drained.mode, ChunkingMode::Smooth);

    let held = policy.decide(
        snapshot(/*queued_lines*/ 8, /*oldest_age_ms*/ 20),
        t0 + Duration::from_millis(120),
    );
    assert_eq!(held.mode, ChunkingMode::Smooth);
    assert_eq!(held.drain_plan, DrainPlan::Single);

    let reentered = policy.decide(
        snapshot(/*queued_lines*/ 8, /*oldest_age_ms*/ 20),
        t0 + Duration::from_millis(320),
    );
    assert_eq!(reentered.mode, ChunkingMode::CatchUp);
    assert_eq!(reentered.drain_plan, DrainPlan::Batch(8));
}

#[test]
// 严重积压可越过再进入冷却，避免延迟继续增长。
fn severe_backlog_can_reenter_during_hold() {
    let mut policy = AdaptiveChunkingPolicy::default();
    let t0 = Instant::now();

    let _ = policy.decide(snapshot(/*queued_lines*/ 8, /*oldest_age_ms*/ 20), t0);
    let _ = policy.decide(
        QueueSnapshot { queued_lines: 0, oldest_age: None },
        t0 + Duration::from_millis(20),
    );

    let severe = policy.decide(
        snapshot(/*queued_lines*/ 64, /*oldest_age_ms*/ 20),
        t0 + Duration::from_millis(120),
    );
    assert_eq!(severe.mode, ChunkingMode::CatchUp);
    assert_eq!(severe.drain_plan, DrainPlan::Batch(64));
}
