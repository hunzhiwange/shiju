use super::*;
pub(super) fn job_row<'a>(app: &'a App, job: &'a CronJobDto) -> Element<'a, Message> { domain::list::job_row(&data(app),job).map(map) }
pub(super) fn jobs_tab(app: &App) -> Element<'_, Message> { domain::list::jobs_tab(&data(app)).map(map) }
