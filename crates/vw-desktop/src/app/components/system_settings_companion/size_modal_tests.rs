// 宿主契约：尺寸草稿与持久化缩放隔离、真实头像绘制、取消/重置 ViewMessage
// 及遮罩阻止底层头像切换；局部弹窗不能替代宿主 preview_scale 投影。
#[test]
fn mini_companion_size_reset_restores_default() {
    use crate::app::{
        App,
        message::view::{ViewMessage, update},
    };
    let mut app = App::new().0;
    app.task_pet_scale = 1.5;
    let _ = update(&mut app, ViewMessage::TaskPetSizeOpen);
    let _ = update(&mut app, ViewMessage::TaskPetSizeReset);
    assert_eq!(app.task_pet_preview_scale(), 1.0);
    assert_eq!(app.task_pet_scale, 1.5);
}
