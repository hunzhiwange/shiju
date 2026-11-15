//! 结果卡片的只读投影；问题请求仍由问题领域拥有。
pub struct ResultInput<'a> {
    pub i18n: desktop_i18n::I18n,
    pub hovered: Option<u64>,
    pub question_request: Option<&'a question_types::Request>,
    pub message_ids: &'a [Option<String>],
}
