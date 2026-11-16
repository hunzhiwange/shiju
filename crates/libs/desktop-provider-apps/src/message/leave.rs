use iced::{Element,Length};
#[derive(Clone, Debug)]
pub enum Choice {
    Cancel,
    Discard,
    Save,
}


#[derive(Clone,Debug)]
pub struct LeaveState<M> {pub dirty:bool,pub pending:Option<Box<M>>,pub saving:bool}
impl<M> Default for LeaveState<M>{fn default()->Self{Self{dirty:false,pending:None,saving:false}}}
pub enum Action<M>{None,Resume(M),SaveEdit,SaveList}
pub fn intercept<M:Clone>(state:&mut LeaveState<M>,message:&M,edits:bool,leaving:bool,busy:bool,draft:bool)->bool {
if edits && !busy && draft {state.dirty=true;}
if leaving && (state.dirty || busy) {if state.pending.is_none(){state.pending=Some(Box::new(message.clone()));} return true;}
false
}
pub fn choose<M>(state:&mut LeaveState<M>,form:&mut Option<crate::state::edit::EditForm>,editor:&mut Option<crate::state::list::Editor>,choice:Choice)->Action<M>{
if state.pending.is_none(){return Action::None}
let busy=form.as_ref().is_some_and(|f|f.saving)||editor.as_ref().is_some_and(|f|f.saving);
match choice {
Choice::Cancel=>{state.pending=None;state.saving=false;Action::None},
Choice::Discard if !busy=>{let pending=state.pending.take();*form=None;*editor=None;*state=Default::default();pending.map_or(Action::None,|m|Action::Resume(*m))},
Choice::Save if !busy=>{state.saving=true;if form.is_some(){Action::SaveEdit}else{Action::SaveList}},
_=>Action::None,
}
}
pub fn resume<M>(state:&mut LeaveState<M>,draft:bool,busy:bool)->Option<M>{
if !draft {state.dirty=false;if state.saving {state.saving=false;return state.pending.take().map(|m|*m)}} else if !busy {state.saving=false;}
None
}
pub fn overlay<'a, M: Clone + 'static>(pending: bool, busy: bool, base: Element<'a, M>, message: impl Fn(Choice)->M) -> Element<'a, M> {
    use desktop_ui_controls::system_settings_common::*;
    use iced::widget::{button, column, row, text};
    if !pending {
        return base;
    }
    let cancel = message(Choice::Cancel);
    let mut discard = button(text("放弃并离开")).padding([9, 14]).style(rounded_action_btn_style);
    let mut save = button(text("保存后离开")).padding([9, 14]).style(primary_action_btn_style);
    if !busy {
        discard = discard.on_press(message(Choice::Discard));
        save = save.on_press(message(Choice::Save));
    }
    let card = settings_modal_card(
        column![
            row![text("未保存的修改").size(20), settings_close_button(cancel.clone())].spacing(16),
            text(if busy {
                "正在保存，请等待结果。"
            } else {
                "离开前是否保存当前修改？"
            }),
            row![
                button(text("取消"))
                    .padding([9, 14])
                    .style(rounded_action_btn_style)
                    .on_press(cancel.clone()),
                discard,
                save
            ]
            .spacing(10),
        ]
        .spacing(18),
    )
    .width(Length::Fixed(480.0));
    settings_modal_overlay(Some(base), cancel, card)
}

pub fn busy(form:Option<&crate::state::edit::EditForm>,editor:Option<&crate::state::list::Editor>)->bool { form.is_some_and(|f|f.saving)||editor.is_some_and(|f|f.saving) }
