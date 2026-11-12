use super::*;
use iced::widget::{column, row};
use ui::{Icon, action, card, disclosure, muted};
impl App {
    pub(super) fn view(&self) -> Element<'_, Msg> {
        let mut nav = column![row![ui::icon(Icon::Layers).width(28).height(28).style(|_,_| widget::svg::Style { color: Some(ui::GOLD) }),
            column![text("YO3").size(26).font(bold()),text("REVIVAL").size(11).color(ui::MUTED)].spacing(1)].spacing(12).align_y(iced::Alignment::Center),widget::Space::new().height(20)].spacing(6);
        nav=nav.push(ui::mode_switch(self.settings.advanced_mode,!self.locked()));
        let pages=[("Play",Icon::Play,Page::Play),("Hosting",Icon::Server,Page::Server),("Activity",Icon::Activity,Page::Activity),("Game setup",Icon::Settings,Page::Settings)];
        for (label,icon,page) in pages {
            let selected=self.page==page;
            nav=nav.push(action(icon,label,Some(Msg::Page(page))).width(Fill).style(move |theme,status| {
                let mut style=button::text(theme,status); if selected { style.background=Some(Color::from_rgb8(36,57,79).into()); } style.border.radius=8.0.into(); style
            }));
        }
        nav=nav.push(widget::Space::new().height(Fill))
            .push(action(Icon::Book,"Help & support",Some(Msg::Support)).width(Fill).style(button::text)).push(widget::rule::horizontal(1))
            .push(button(column![text(format!("Version {}",yo3_tunnel::BUILD_VERSION)).size(12),text(match self.update_status { UpdateStatus::Checked(updates::Decision::Available(_))=>"Update available", _=>"About & updates" }).size(11).color(ui::MUTED)].spacing(5)).on_press(Msg::Page(Page::About)).padding([10,4]).width(Fill).style(button::text));
        let content=match self.page {
            Page::About=>self.about_view(),_ if !self.settings.advanced_mode=>self.normal_view(),
            Page::Play=>self.play_view(),Page::Server=>self.server_view(),Page::Activity=>self.activity_view(),Page::Settings=>self.settings_view() };
        let mut main=column![].spacing(16).padding(28).width(Fill).height(Fill);
        if let Some((release,transfer))=&self.transfer {
            let detail=if transfer.cancelled() { "Cancelling…".into() } else if transfer.verifying() { "Verifying and unpacking…".into() } else { format!("{} of {}",size(transfer.bytes()),size(release.asset.bytes)) };
            main=main.push(ui::notice(column![row![self.spinner(),text(format!("Updating to {}",release.tag)).size(16).width(Fill),action(Icon::Close,"Cancel download",(!transfer.cancelled()).then_some(Msg::CancelUpdate)).style(button::text)].spacing(10),widget::progress_bar(0.0..=release.asset.bytes as f32,transfer.bytes() as f32).girth(5),muted(detail)].spacing(12),ui::Tone::Info));
        } else if self.applying || self.staged.is_some() { main=main.push(ui::notice(column![row![self.spinner(),text("Restarting into the update…").size(16)].spacing(10),muted("Waiting for the game/server to close and the new launcher to become ready.")].spacing(8),ui::Tone::Info)); }
        let base=if self.settings.advanced_mode {
            row![container(nav).width(Fixed(202.0)).height(Fill).padding(20).style(|_|container::Style{background:Some(Color::from_rgb8(19,26,37).into()),..Default::default()}),main.push(content)].into()
        } else { main.push(content).into() };
        if self.support {
            let mut body=column![heading("Help & support","Setup help and tools for reporting a problem."),
                action(Icon::Book,"Read the guide",Some(Msg::Help)).style(button::text),
                widget::rule::horizontal(1),
                text("Share a problem").size(17).font(bold()),
                muted("Prepare a GitHub bug report with your version, system details and diagnostic logs."),
                action(Icon::External,"Report a problem…",(!self.locked()).then_some(Msg::ReportBug)).style(button::primary),
                row![action(Icon::Download,"Export logs…",(!self.locked()).then_some(Msg::ExportLogs)).style(button::text),
                    action(Icon::Copy,"Copy logs",Some(Msg::CopyLogs)).style(button::text)].spacing(12),
                widget::rule::horizontal(1),
                text("Game recovery").size(17).font(bold()),
                muted("For repeated startup problems or broken settings, review a backed-up reset of the game's local files."),
                action(Icon::Refresh,"Reset local game data…",(!self.locked()).then_some(Msg::Troubleshoot)).style(button::text)].spacing(14);
            if self.locked(){body=body.push(ui::inline_status("Export and recovery tools are available when the current operation finishes. You can still copy the logs.",ui::Tone::Info));}
            if self.feedback_visible() && self.failed {body=body.push(self.page_heading("",""));}
            return ui::modal_actions(base,body.into(),row![action(Icon::Close,"Close",Some(Msg::CancelClose)).style(button::text)].into());
        }
        if self.review_update && let UpdateStatus::Checked(updates::Decision::Available(release))=&self.update_status {
            let failed=self.failed && self.feedback_visible();
            let header=column![text("Install this update?").size(24).font(bold()),
                row![text(format!("{} → {}",yo3_tunnel::BUILD_VERSION,release.tag)).size(16).width(Fill),muted(size(release.asset.bytes)).width(iced::Length::Shrink)].spacing(12)].spacing(6);
            let mut consequences=column![ui::notice(column![
                ui::inline_status("Games started here will close. Your hosted server will restart and disconnect its players. Close separately opened games first.",ui::Tone::Warning),
                muted("Settings, accounts and decks stay in place. The previous version remains available if startup fails.")].spacing(8),ui::Tone::Warning)].spacing(10);
            let diagnosis=failed.then(||self.error_evidence()).flatten();let has_diagnosis=diagnosis.is_some();
            if failed {
                let mut error=column![row![ui::inline_status(self.error_summary(),ui::Tone::Danger),
                    action(Icon::Copy,"Copy error",Some(Msg::CopyError)).style(button::text)].spacing(10).align_y(iced::Alignment::Center)].spacing(6);
                if let Some(diagnosis)=diagnosis {error=error.push(diagnosis);}
                consequences=consequences.push(ui::notice(error,ui::Tone::Danger))
                    .push(muted("Copy error for the full report, or open the release page manually."));
            }
            return ui::update_review(base,header.into(),ui::release_notes_panel(&self.release_notes,if has_diagnosis{60.0}else if failed{100.0}else{190.0}),consequences.into(),
                row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)).style(button::text),
                    action(Icon::Download,"Download & restart",Some(Msg::ConfirmUpdate)).style(button::primary)].spacing(12).into());
        }
        if self.report_options && self.bug_report.is_some() {return self.bug_report_view(base);}
        if self.report_options {
            let mut body=column![heading("Export logs","Choose which retained sessions to include."),
                muted("Logs are captured automatically and retained for up to 14 days or 100 MiB. Credentials, databases and game files are always excluded.")].spacing(16);
            if self.report_busy {body=body.push(muted(if self.report_all{"All retained sessions selected"}else{"Current session selected"}));}
            else {body=body.push(widget::radio("Current session",false,Some(self.report_all),Msg::PrepareReport)).push(widget::radio("All retained sessions",true,Some(self.report_all),Msg::PrepareReport));}
            body=body.push(ui::report_filter(self.report_remove_personal,!self.report_busy))
                .push(ui::inline_status(if self.report_remove_personal {"Omits game, packet and engine traces and player activity; masks common user paths and network addresses. This reduces debugging detail. Review before sharing."}else{"Full report: may include player names, chat, decks, file paths and network addresses. Enable filtering if you prefer to leave those out."},if self.report_remove_personal{ui::Tone::Info}else{ui::Tone::Warning}));
            if let Some(report)=&self.report {
                let p=&report.preview; let date=|n|chrono::DateTime::from_timestamp(n as i64,0).map(|d|d.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(||"No sessions".into());
                body=body.push(text(format!("{} files · {} in source logs",p.files,size(p.bytes))).size(17)).push(muted(format!("{} to {}",date(p.oldest),date(p.newest))));
            } else if !self.report_busy && self.report_feedback.is_none() {body=body.push(muted("No preview available. Select a scope to retry."));}
            if self.report_busy {body=body.push(row![self.spinner(),muted(if self.report.is_some(){"Choose a ZIP location, then wait for the report to save…"}else{"Preparing preview…"})].spacing(10));}
            if let Some(result)=&self.report_feedback {let message=result.as_ref().unwrap_or_else(|e|e);let tone=if result.is_err(){ui::Tone::Danger}else{ui::Tone::Success};body=body.push(ui::notice(ui::inline_status(message,tone),tone));
                if result.is_err() {body=body.push(action(Icon::Copy,"Copy export error",Some(Msg::CopyReportError)).style(button::text));}
            }
            if !self.report_busy && self.report.is_none() && self.report_feedback.as_ref().is_some_and(Result::is_err) {
                body=body.push(action(Icon::Refresh,"Retry preview",Some(Msg::PrepareReport(self.report_all))).style(button::text));
            }
            if self.failed && self.feedback_visible() {body=body.push(self.page_heading("",""));}
            body=body.push(muted("Filtering affects this ZIP only. Your original logs stay available. Nothing is uploaded."));
            return ui::modal_actions(base,body.into(),row![action(Icon::Close,"Close",Some(Msg::CancelClose)).style(button::text),
                action(Icon::Download,"Save ZIP…",(!self.report_busy && self.report.is_some()).then_some(Msg::SaveLogs)).style(button::primary)].spacing(12).into());
        }
        if let Some(observed)=&self.observed {
            let compatible=observed.info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION,yo3_tunnel::MOD_ABI);
            let body=column![heading("Confirm this server","Compare this identity with the host before connecting."),text(&self.address).size(18),
                ui::status_badge(format!("{} · {}",observed.info.release,if compatible.is_ok(){"Compatible"}else{"Update required"}),if compatible.is_ok(){ui::Tone::Success}else{ui::Tone::Danger}),
                muted("Ask the host to confirm this code. An imported invitation already carries the expected identity."),container(text(&observed.identity).font(iced::Font::MONOSPACE).size(12)).padding(14).style(container::rounded_box),
                if let Err(error)=&compatible{ui::inline_status(error.to_string(),ui::Tone::Danger)}else{ui::inline_status("Your account key has not been sent.",ui::Tone::Info)}].spacing(20);
            return ui::modal_actions(base,body.into(),row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)).style(button::text),
                action(Icon::Shield,"Trust this server",compatible.is_ok().then_some(Msg::TrustIdentity)).style(button::primary)].spacing(12).into());
        }
        if self.remove_server.is_some() {
            return ui::modal_actions(base,column![heading("Remove this connection?","The account and its game data remain on the server."),ui::inline_status("This removes its saved address and credentials from this launcher. You will need the invitation to add it again.",ui::Tone::Warning)].spacing(20).into(),
                row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)).style(button::text),action(Icon::Close,"Remove connection",Some(Msg::ConfirmRemoveServer)).style(button::danger)].spacing(12).into());
        }
        if let Some(dialog)=&self.game_data_dialog {
            let mut body=match dialog {
                GameDataDialog::Checking {..}=>column![heading("Troubleshooting","Checking your local game data before offering a reset."),
                    row![self.spinner(),muted("Finding the game's data folder…")].spacing(10)],
                GameDataDialog::Problem(error)=>column![heading("Troubleshooting","Local game data could not be prepared for a reset."),
                    ui::notice(ui::inline_status(error.lines().next().unwrap_or(error),ui::Tone::Danger),ui::Tone::Danger)],
                GameDataDialog::Complete=>column![heading("Local game data reset","Your backup is ready. Close this dialog and choose Play when you are ready."),
                    ui::inline_status("The game will rebuild the reset files next time it opens.",ui::Tone::Success)],
                GameDataDialog::Ready {settings,preview,keep_players}=>{
                    let mut details=column![heading("Reset local game data?","Troubleshooting for broken client settings or repeated startup problems."),
                        muted("Local data folder"),text(display_path(&preview.directory)).font(iced::Font::MONOSPACE).size(12),
                        checkbox(*keep_players).label("Keep player files and saved decks").on_toggle(Msg::KeepGameDataPlayers),
                        muted(if *keep_players {"Resets game options and the original game's cached login data. Every local player file, saved deck and selected deck stays in place."}
                            else {"Resets all local game data. Player files, saved decks and the selected deck move into the backup. The game creates fresh local files next time you play."}),
                        muted("A dated backup is created before any files are changed. Server accounts, cards and BP, and your Revival logins stay in place."),
                        muted("Close every copy of Yu-Gi-Oh! ONLINE 3, including games opened outside Revival.")].spacing(14);
                    if !keep_players {details=details.push(ui::notice(ui::inline_status("Your saved deck files will also be reset. Keep the backup if you want to restore them.",ui::Tone::Warning),ui::Tone::Warning));}
                    if self.game_running {details=details.push(ui::inline_status("Close the game before resetting its data.",ui::Tone::Warning));}
                    else if self.changed(Section::Game) {details=details.push(ui::inline_status("Close this dialog and let the automatic game-folder check finish, or restore your previous path, before resetting local data.",ui::Tone::Info));}
                    else if settings!=&self.settings {details=details.push(ui::inline_status("Game setup changed. Choose Check again to review the current folder.",ui::Tone::Warning));}
                    else if if *keep_players {!preview.settings_present}else{!preview.full_present} {details=details.push(ui::inline_status("There are no local files to reset for this selection.",ui::Tone::Neutral));}
                    details
                }
            }.spacing(16);
            if self.failed && self.feedback_visible() {
                if !matches!(dialog,GameDataDialog::Problem(_)) {
                    let mut error=column![ui::inline_status(self.error_summary(),ui::Tone::Danger)].spacing(6);
                    if let Some(diagnosis)=self.error_evidence() {error=error.push(diagnosis);}
                    body=body.push(ui::notice(error,ui::Tone::Danger));
                }
                body=body.push(action(Icon::Copy,"Copy error",Some(Msg::CopyError)).style(button::text));
            }
            if let Some(backup)=&self.game_data_backup {
                body=body.push(widget::rule::horizontal(1)).push(muted(if matches!(dialog,GameDataDialog::Complete){"Backup folder"}else{"Last reset backup"}))
                    .push(text(display_path(backup)).font(iced::Font::MONOSPACE).size(12))
                    .push(action(Icon::Folder,"Open backup folder",Some(Msg::OpenGameDataBackup)).style(button::text));
            }
            if self.backend.is_some() && !self.locked() && (matches!(dialog,GameDataDialog::Problem(_)) || matches!(dialog,GameDataDialog::Ready {settings,..} if settings!=&self.settings)) {
                body=body.push(action(Icon::Refresh,"Check again",Some(Msg::Troubleshoot)).style(button::text));
            }
            let mut controls=row![action(Icon::Close,if matches!(dialog,GameDataDialog::Ready {..}|GameDataDialog::Checking {..}){"Cancel"}else{"Close"},Some(Msg::CancelClose)).style(button::text)].spacing(12);
            if matches!(dialog,GameDataDialog::Ready {..}) {
                controls=controls.push(action(Icon::Refresh,"Back up & reset",self.game_data_reset_request().map(|_|Msg::ConfirmGameDataReset)).style(button::danger));
            }
            return ui::modal_actions(base,body.into(),controls.into());
        }
        if self.closing {
            if !self.settings.advanced_mode {
                let mut body=column![heading("Close YO3 Revival?","A game started here will close.")].spacing(16);
                if self.busy || self.feedback_visible() {body=body.push(self.page_heading("",""));}
                if self.status.is_some(){body=body.push(ui::inline_status("Your local server will stop and connected players will be disconnected.",ui::Tone::Warning));}
                return ui::modal_actions(base,body.into(),
                    row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)).style(button::text),action(Icon::Close,"Close app",(!self.locked()).then_some(Msg::Quit(true))).style(button::primary)].spacing(12).into());
            }
            let mut body=column![heading("Close YO3 Revival?","A game started here will close.")].spacing(16);
            if self.busy || self.feedback_visible() {body=body.push(self.page_heading("",""));}
            if self.status.is_some() {
                body=body.push(checkbox(self.keep_hosting_on_exit).label("Keep hosting after I close Revival").on_toggle_maybe((!self.locked()).then_some(Msg::KeepHosting)))
                    .push(ui::inline_status(if self.keep_hosting_on_exit {"Your server and its connected players stay online."}else{"Your server will stop and its connected players will be disconnected."},if self.keep_hosting_on_exit{ui::Tone::Info}else{ui::Tone::Warning}));
            }
            let stop=self.status.is_some() && !self.keep_hosting_on_exit;
            return ui::modal_actions(base,body.into(),row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)).style(button::text),
                action(Icon::Close,"Close app",(!self.locked()).then_some(Msg::Quit(stop))).style(button::primary)].spacing(12).into());
        }
        base
    }
    fn error_evidence(&self)->Option<Element<'_,Msg>> {
        self.error_diagnosis().map(|diagnosis|text(diagnosis).size(13).color(ui::MUTED)
            .wrapping(text::Wrapping::WordOrGlyph).width(Fill).into())
    }
    fn missing_file_list(&self)->Element<'_,Msg> {
        column(self.setup_missing_files.iter().map(|path|row![text("•").color(ui::BAD),
            text(display_path(path)).size(13).font(iced::Font::MONOSPACE).wrapping(text::Wrapping::WordOrGlyph).width(Fill)]
            .spacing(10).into())).spacing(8).width(Fill).into()
    }
    fn page_heading(&self,title:&'static str,description:&'static str)->Element<'_,Msg>{
        let mut body=column![].spacing(16);
        if !title.is_empty() { body=body.push(heading(title,description)); }
        if self.busy {body=body.push(ui::notice(row![self.spinner(),text(&self.operation_label).size(13).color(ui::tone_color(ui::Tone::Info)).width(Fill)].spacing(10).align_y(iced::Alignment::Center),ui::Tone::Info));}
        if self.feedback_visible() {
            let field_error=self.failed && self.game_field_error().is_some() && !self.modal_open()
                && matches!(self.failure_setup,Some(yo3_desktop::Readiness::GameProblem(_)))
                && (!self.settings.advanced_mode || self.page==Page::Settings);
            let tone=if self.failed{ui::Tone::Danger}else{ui::Tone::Success};
            let mut feedback=column![].spacing(12);
            if !field_error {
                let summary=if self.failed && self.failure_setup.is_some() && !self.setup_missing_files.is_empty(){format!("Missing required files ({})",self.setup_missing_files.len())}else{self.error_summary()};
                let mut header=row![ui::inline_status(summary,tone)].spacing(10).align_y(iced::Alignment::Center);
                if self.failed {header=header.push(widget::tooltip(button(ui::icon(Icon::Copy)).padding(4).style(button::text).on_press(Msg::CopyError),
                    container(text("Copy error").size(12)).padding(8).style(container::rounded_box),widget::tooltip::Position::Top).snap_within_viewport(true));}
                if self.failed && !self.modal_open() {header=header.push(widget::tooltip(button(ui::icon(Icon::External)).padding(4).style(button::text).on_press_maybe((!self.locked()).then_some(Msg::ReportBug)),
                    container(text("Report a problem").size(12)).padding(8).style(container::rounded_box),widget::tooltip::Position::Top).snap_within_viewport(true));}
                feedback=feedback.push(header.push(button(ui::icon(Icon::Close)).padding(4).style(button::text).on_press(Msg::DismissNotice)));
            }
            if self.failed {
                let missing_only=self.failure_setup.is_some() && !self.setup_missing_files.is_empty();
                if !field_error {
                    if self.failure_setup.is_some() && !self.setup_missing_files.is_empty() {
                        feedback=feedback.push(container(scrollable(self.missing_file_list()).height(iced::Length::Shrink)).max_height(112))
                            .push(muted("Antivirus may have removed these files. If so, restore them and exclude the affected Revival or game folder from scans."));
                    } else {
                        if let Some(diagnosis)=self.error_evidence() {feedback=feedback.push(diagnosis);}
                        feedback=feedback.push(muted(match &self.failure_setup {
                        _ if self.report_options=>"Your original error is kept here. Close this dialog to return to it.",
                        Some(yo3_desktop::Readiness::GameProblem(_))=>"Choose a complete original game folder.",
                        Some(yo3_desktop::Readiness::ComponentProblem(_))=>"Extract the complete matching Desktop download, then retry.",
                        _ if self.closing=>"Choose Cancel to return to the app, or try Close app again.",
                        _=>"Export logs when asking for help.",
                        }));
                    }
                }
                if field_error && missing_only {
                    feedback=feedback.push(container(scrollable(self.missing_file_list()).height(iced::Length::Shrink)).max_height(112))
                        .push(muted("Antivirus may have removed these files. If so, restore them and exclude the affected Revival or game folder from scans."));
                }
                let saved=self.failure_setup.is_some() && !self.modal_open() && self.changed(Section::Game) && !self.settings.game_dir.trim().is_empty();
                let download=matches!(self.setup_result,Some(yo3_desktop::Readiness::ComponentProblem(_))) && !self.modal_open();
                let mut controls=row![].spacing(8).align_y(iced::Alignment::Center);
                if saved || download {controls=controls.push(widget::Space::new().width(Fill));}
                if saved {controls=controls.push(action(Icon::Refresh,"Use saved folder",(!self.locked()).then_some(Msg::CancelSettings(Section::Game))).style(button::text));}
                if download {controls=controls.push(action(Icon::Download,"Download Desktop…",Some(Msg::ReleasePage)));}
                if saved || download {feedback=feedback.push(controls);}
            }
            if field_error {body=body.push(feedback);}
            else {body=body.push(ui::notice(feedback,tone));}
        }
        body.into()
    }
    pub(super) fn spinner(&self)->Element<'_,Msg> {
        // Standard text widgets keep animation inside their layout bounds on the software renderer.
        let active=(self.pulse*3.0/std::f32::consts::TAU) as usize;
        row((0..3).map(|dot|text("•").size(16).color(if dot==active{ui::tone_color(ui::Tone::Info)}else{ui::MUTED}).into())).spacing(2).width(28).into()
    }
    fn normal_view(&self) -> Element<'_, Msg> {
        column![row![ui::icon(Icon::Layers).width(26).height(26),text("YO3 Revival").size(25).font(bold()).width(Fill),
            ui::mode_switch(false,!self.locked())].spacing(10).align_y(iced::Alignment::Center),
            widget::rule::horizontal(1),self.normal_content()]
            .spacing(16).height(Fill).into()
    }
    fn normal_content(&self) -> Element<'_, Msg> {
        let editable=self.backend.is_some() && !self.locked() && !self.game_running;
        let field_error=self.game_field_error();let invalid=field_error.is_some();
        let mut body=column![].spacing(16);
        if self.backend.is_none() {body=body.push(column![muted("Your settings or logs could not be opened. Check that the data folder is writable, then retry. Your saved data will be kept."),
            row![action(Icon::Refresh,"Retry app setup",(!self.locked()).then_some(Msg::RetrySetup)),action(Icon::Folder,"Open data folder",Some(Msg::OpenData))].spacing(12)].spacing(8));}
        let mut game=column![text("Game folder").size(15).font(bold()),
            row![text_input("Choose the folder containing yo3.exe",&self.draft.game_dir).padding(13).width(Fill)
                .on_input_maybe(editable.then_some(|s|Msg::Input(Field::Game,s))).style(move |theme,status|ui::input_style(theme,status,invalid)),
                action(Icon::Folder,"Browse…",editable.then_some(Msg::Browse(Field::Game)))].spacing(12).align_y(iced::Alignment::Center)].spacing(8);
        if let Some(error)=field_error {game=game.push(ui::inline_status(error,ui::Tone::Danger));}
        body=body.push(game.push(muted("Choose your original Yu-Gi-Oh! ONLINE 3 installation. It is checked and saved automatically. Game files are not included.")));
        let status=if invalid{"Check the highlighted game folder."}else{self.play_status()};
        body=body.push(row![ui::inline_status(status,self.play_tone()),self.play_action()].spacing(16).height(52).align_y(iced::Alignment::Center));
        body=body.push(muted("Play rechecks the required files, then opens your game and prepares everything automatically."));
        if self.picking {body=body.push(row![muted("No file dialog? You can paste the game folder path instead."),action(Icon::Close,"Use path instead",Some(Msg::UsePath))].spacing(12).align_y(iced::Alignment::Center));}
        if self.selected!=ServerChoice::Local || self.editor {
            body=body.push(row![muted(if self.editor {"Finish or cancel your connection in Advanced before playing.".into()}else{format!("Playing on {}",self.selected)}),
                action(Icon::Settings,"Change in Advanced",(!self.locked()).then_some(Msg::Mode(true))).style(button::text)].spacing(12).align_y(iced::Alignment::Center));
        }
        // Feedback belongs below the launch controls so expiring notices cannot move Play mid-click.
        if self.feedback_visible() { body=body.push(self.page_heading("","")); }
        if matches!(self.setup_result,Some(yo3_desktop::Readiness::ComponentProblem(_))) && !self.feedback_visible() {
            body=body.push(action(Icon::Download,"Download Desktop…",Some(Msg::ReleasePage)).style(button::text));
        }
        let logs=friendly_logs(&self.shown_logs());let logs=if logs.is_empty(){"Checks and startup details will appear here. Full traces are saved automatically for export.".into()}else{logs};
        body=body.push(row![container(ui::label_help("Logs","Compact activity: repeated lines are grouped and Wine diagnostics are hidden. Advanced → Activity shows technical output; Export logs includes full diagnostics.")).width(Fill),
            action(Icon::Download,"Export logs…",(!self.locked()).then_some(Msg::ExportLogs)).style(button::text)].spacing(10).align_y(iced::Alignment::Center))
            .push(ui::log_panel(logs).max_height(160))
            .push(row![button(text(format!("Version {}{}",yo3_tunnel::BUILD_VERSION,match &self.update_status {UpdateStatus::Checking=>" · Checking for updates…",UpdateStatus::Failed(_)=>" · Update check unavailable",UpdateStatus::Checked(updates::Decision::Available(_))=>" · Update available",_=>""})).size(12)).on_press(Msg::Page(Page::About)).style(button::text),
                widget::Space::new().width(Fill),
                action(Icon::Book,"Help & support",Some(Msg::Support)).style(button::text)].spacing(12).align_y(iced::Alignment::Center));
        // Keep details, recovery actions and logs reachable at the minimum window size.
        // Reserve the scrollbar gutter even when it is hidden, keeping Play in the same position.
        scrollable(container(body.width(Fill)).padding(iced::Padding {right:22.0,..Default::default()}).width(Fill)).width(Fill).height(iced::Length::Shrink).into()
    }
    fn play_action(&self)->Element<'_,Msg> {
        let loading=self.play_loading();let danger=self.game_running && !loading;
        let symbol:Element<'_,Msg>=if loading{
            let active=(self.pulse*3.0/std::f32::consts::TAU) as usize;
            row((0..3).map(|dot|text("•").size(16).color(Color {a:if dot==active{1.0}else{0.35},..Color::from_rgb8(15,20,29)}).into())).spacing(2).width(28).into()
        }else{ui::icon(if self.game_running{Icon::Stop}else if self.play_label()=="Retry check"{Icon::Refresh}else{Icon::Play}).into()};
        button(container(row![symbol,text(self.play_label()).size(14)].spacing(9).align_y(iced::Alignment::Center)).center_x(Fill).center_y(Fill))
            .on_press_maybe(self.play_message()).width(160).height(46).padding(10)
            .style(move |theme,status| {
                let status=if loading {button::Status::Active}else{status};
                if danger {button::danger(theme,status)}else{button::primary(theme,status)}
            }).into()
    }
    fn play_view(&self) -> Element<'_, Msg> {
        if self.editor {
            return column![self.page_heading(if self.editing_id.is_some() {"Edit server"} else {"Add server"},"Save this connection to make it available on Play."),
                self.connection_editor()].spacing(24).height(Fill).into();
        }
        let mut body = column![self.page_heading("Play", "Choose a server, then launch your game.")].spacing(28);
        if let UpdateStatus::Checked(updates::Decision::Available(release)) = &self.update_status {
            body = body.push(row![muted(format!("Version {} is available", release.tag)),
                action(Icon::Download, "View update", Some(Msg::Page(Page::About))).style(button::text)]
                .spacing(12).align_y(iced::Alignment::Center));
        }
        let editable = !self.locked() && !self.game_running;
        let mut servers = vec![ServerChoice::Local]; servers.extend(self.servers.iter().map(ServerChoice::saved));
        let mut connection = column![row![
            ui::label_help("Server", "This computer starts your own local server. Add a server to join a host by address, or import the private invitation they sent you."),
            widget::Space::new().width(Fill), action(Icon::Plus, "Add server", editable.then_some(Msg::AddServer)).style(button::text)]
            .align_y(iced::Alignment::Center)].spacing(14);
        if !editable {
            connection = connection.push(container(text(self.selected.to_string()).color(ui::MUTED)).padding(12).width(Fill).style(container::rounded_box));
        } else {
            connection = connection.push(pick_list(servers, Some(self.selected.clone()), Msg::Select).width(Fill).padding(12));
        }
        if self.selected == ServerChoice::Local {
            let names = account_names(&self.accounts);
            if !names.is_empty() {
                connection = connection.push(ui::label_help("Your account", "This is the account used to play. Each account can have several players; choose your player inside the game. Managing another account under Hosting does not change this selection."));
                if !editable { connection = connection.push(container(text(&self.account).color(ui::MUTED)).padding(12).width(Fill).style(container::rounded_box)); }
                else { connection = connection.push(pick_list(names, (!self.account.is_empty()).then_some(self.account.clone()), Msg::AccountSelect).width(Fill).padding(12)); }
            }
            connection = connection.push(muted("Your server starts automatically when you play."));
        } else {
            connection = connection.push(row![action(Icon::Shield, "Test connection", (!self.locked()).then_some(Msg::TestConnection)).style(button::text),
                widget::Space::new().width(Fill),action(Icon::Settings, "Edit server", editable.then_some(Msg::EditServer)).style(button::text)].spacing(8));
        }
        if self.game_running { connection = connection.push(muted("Close the game to change its server or account.")); }
        else if self.locked() {connection=connection.push(muted("Server and account selection will be available when the current operation finishes."));}
        if let Some(info) = &self.checked {
            connection = connection.push(ui::status_badge(format!("Server {} · {}", info.release, if info.ready { "Ready" } else { "Rules engine unavailable" }),if info.ready{ui::Tone::Success}else{ui::Tone::Danger}));
        }
        body = body.push(connection).push(widget::rule::horizontal(1));
        let mut launch = column![row![ui::inline_status(self.play_status(),self.play_tone()), self.play_action()].spacing(16).align_y(iced::Alignment::Center)].spacing(12);
        if !self.setup_in_progress() && matches!(self.setup_result, None | Some(yo3_desktop::Readiness::GameProblem(_))) {
            launch = launch.push(action(Icon::Folder, "Review game setup", Some(Msg::Page(Page::Settings))).style(button::text));
        }
        body = body.push(launch).push(muted("In the game: ONLINE → choose your player. Your first starting deck is prepared automatically. Use Main Menu → Edit Deck to customise it."));
        page_scroll(body)
    }
    fn connection_editor(&self) -> Element<'_, Msg> {
        let mut body = column![].spacing(16);
        if self.editing_id.is_none() {
            body = body.push(row![muted("Joining a friend? Import their private invitation to check the server and your account automatically."),
                action(Icon::Upload, "Import invitation…", (!self.locked()).then_some(Msg::Import)).style(button::text)].spacing(12).align_y(iced::Alignment::Center))
                .push(widget::rule::horizontal(1));
        }
        if self.invitation_checking {
            body = body.push(row![self.spinner(), text("Checking the invitation's server, compatibility and account…").size(13).color(ui::tone_color(ui::Tone::Info)).width(Fill)]
                .spacing(10).align_y(iced::Alignment::Center));
        } else if self.invitation_imported && self.identity.is_some() {
            if self.invitation_verified {
                body = body.push(ui::status_badge("Invitation verified · ready to save".into(),ui::Tone::Success));
            } else {
                body = body.push(row![ui::inline_status("Invitation not verified. Make sure the host's server is running, then retry the connection check.",ui::Tone::Warning),
                    action(Icon::Refresh, "Retry check", (!self.locked()).then_some(Msg::CheckInvitation)).style(button::text)]
                    .spacing(12).align_y(iced::Alignment::Center));
            }
        }
        body = body.push(self.field("Server address", "Hostname or IP, optionally with a port", &self.address, Field::Address));
        if self.identity.is_some() {
            if !self.invitation_imported { body = body.push(ui::status_badge("Server identity approved".into(),ui::Tone::Success)); }
            let key_problem=if !self.key.is_ascii() {Some("Use the host's 20-character account key, with letters, numbers and ordinary ASCII characters.")}
                else if self.key.trim().len()>20 {Some("The account key is too long. Paste the exact 20-character key supplied by the host.")}
                else if !self.key.trim().is_empty() && self.key.trim().len()<20 {Some("The account key is incomplete. Paste all 20 characters supplied by the host.")}
                else {None};
            body = body
                .push(self.field("Connection name", "A name for this server", &self.label, Field::Label))
                .push(column![ui::label_help("Account key", "The host supplies this private 20-character key with your account. Keep it secret. It replaces the game's original username/password login."),
                    text_input("Private key from the host", &self.key).secure(true).padding(12)
                        .on_input_maybe((!self.locked() && !self.modal_open()).then_some(|s| Msg::Input(Field::Key, s)))].spacing(8));
            if let Some(problem)=key_problem {
                body=body.push(ui::inline_status(problem,ui::Tone::Warning));
            } else if self.key.trim().is_empty() {
                body = body.push(muted("Enter the complete 20-character account key supplied by the host to save this connection."));
            }
        } else {
            body = body.push(muted(if self.address.trim().is_empty() {
                "Enter the host's address, then check the server. You will confirm its identity before adding your private account key."
            } else { "Check the server and compare its identity with the host before entering an account key." }));
        }
        let mut footer=row![].spacing(12).align_y(iced::Alignment::Center);
        if self.editing_id.is_some() {footer=footer.push(action(Icon::Close,"Remove connection",(!self.locked() && !self.game_running).then_some(Msg::RemoveServer)).style(button::danger));}
        footer=footer.push(widget::Space::new().width(Fill)).push(action(Icon::Close,"Cancel",(!self.locked()).then_some(Msg::AddServer)).style(button::text));
        footer=footer.push(if self.identity.is_some() {
            action(Icon::Check,"Save connection",(!self.locked() && self.key.is_ascii() && self.key.trim().len()==20 && (!self.invitation_imported || self.invitation_verified)).then_some(Msg::SaveServer)).style(button::primary)
        } else {action(Icon::Shield,"Check server",(!self.locked() && !self.address.trim().is_empty()).then_some(Msg::Discover)).style(button::primary)});
        page_scroll(body.push(widget::rule::horizontal(1)).push(footer))
    }
    fn server_view(&self) -> Element<'_, Msg> {
        let content: Element<'_, Msg> = match self.host_tab {
            HostTab::Overview => scrollable(self.host_overview()).spacing(12).width(Fill).height(Fill).into(),
            HostTab::Invitations => scrollable(self.accounts_view()).spacing(12).width(Fill).height(Fill).into(),
            HostTab::Advanced => {
                let mut body=column![scrollable(self.host_advanced()).spacing(12).width(Fill).height(Fill)].spacing(16).height(Fill);
                if self.changed(Section::Hosting) {body=body.push(self.save_actions(Section::Hosting));}
                body.into()
            },
        };
        column![self.page_heading("Hosting", "Host independently while you play here or join another server."),
            row![ui::tab("Overview", Msg::HostTab(HostTab::Overview), self.host_tab == HostTab::Overview),
                ui::tab("Players & invitations", Msg::HostTab(HostTab::Invitations), self.host_tab == HostTab::Invitations),
                ui::tab("Advanced", Msg::HostTab(HostTab::Advanced), self.host_tab == HostTab::Advanced)].spacing(8),
            content].spacing(20).height(Fill).into()
    }
    fn host_overview(&self)->Element<'_,Msg>{
        let running=self.status.is_some();let status=self.status.as_ref();
        let engine_problem=status.and_then(|s|s["engine_problem"].as_str()).filter(|problem|!problem.trim().is_empty());
        let (label,tone)=if engine_problem.is_some(){("Needs attention",ui::Tone::Danger)}else if running{("Running",ui::Tone::Success)}else{("Stopped",ui::Tone::Neutral)};
        let mut body=column![card(column![row![text(if running{"Running on this computer"}else{"Ready to host"}).size(22).font(bold()).width(Fill),ui::status_badge(label.into(),tone)].spacing(12),
            row![metric("Players online",status.and_then(|s|s["online"].as_u64()).unwrap_or(0)),metric("Active duels",status.and_then(|s|s["duels"].as_u64()).unwrap_or(0))].spacing(24),
            row![muted(if running {"Your server stays available when you close the game. Stop it here when everyone has finished."}else{"Start your server to play locally or invite friends. It uses your saved hosting settings."}),
                if running{action(Icon::Stop,"Stop server",(!self.busy && !self.applying).then_some(Msg::Stop)).style(button::danger)}else{action(Icon::Play,"Start server",(!self.locked()).then_some(Msg::Start)).style(button::primary)}].spacing(16).align_y(iced::Alignment::Center)].spacing(20))].spacing(20);
        if let Some(problem)=engine_problem {
            body=body.push(ui::notice(column![ui::inline_status("The rules engine is unavailable",ui::Tone::Danger),
                text(problem).size(13),muted("Players cannot duel until the engine is restored. Check the details in Activity or export logs for help.")].spacing(8),ui::Tone::Danger));
        }
        if self.changed(Section::Hosting) {
            body=body.push(ui::inline_status("Unsaved hosting changes. Save them in Advanced before starting with the new settings.",ui::Tone::Warning));
        }
        let mut sharing=column![ui::label_help("Invite friends", "Create a separate account for each friend, then send them a private invitation. A sharing address must be reachable from their computer."),muted("Set a reachable address, then create an account and send its private invitation to your friend.")].spacing(14);
        match yo3_desktop::network::share_address(&self.settings){Ok(Some((address,_)))=>sharing=sharing.push(row![text(format!("Sharing address: {address}")).size(14).width(Fill),action(Icon::Copy,"Copy",Some(Msg::CopyAddress)).style(button::text)].spacing(12))
            .push(action(Icon::Users,"Create invitations",Some(Msg::HostTab(HostTab::Invitations))).style(button::text)),_=>sharing=sharing.push(muted("A sharing address is not configured. localhost is for local play."))
            .push(action(Icon::Settings,"Configure sharing",Some(Msg::HostTab(HostTab::Advanced))).style(button::text))};
        sharing=sharing.push(action(Icon::Book,"Connection help",Some(Msg::NetworkHelp)).style(button::text));body=body.push(widget::rule::horizontal(1)).push(sharing);
        if let Some(status)=status {
            if let Some(players)=status["players"].as_array().filter(|p|!p.is_empty()){let mut people=column![text("Online now").size(18).font(bold())].spacing(12);
                for p in players{let duelling=p["duelling"].as_bool().unwrap_or(false);people=people.push(row![text(p["name"].as_str().unwrap_or("Player")).width(Fill),ui::status_badge(if duelling{"Duelling"}else{"Online"}.into(),if duelling{ui::Tone::Info}else{ui::Tone::Success})].spacing(12));}body=body.push(people);}
            else {body=body.push(muted("No players are connected yet. You can join from Play while this server keeps running."));}
        }
        body.into()
    }
    fn host_advanced(&self)->Element<'_,Msg>{
        let selected=self.bindings.iter().find(|b|b.interface_id==self.draft.bind_interface && b.address.to_string()==self.draft.bind_address).cloned();
        let host_problem=(!self.draft.host_address.trim().is_empty() && self.draft.host_address!=self.settings.host_address)
            .then(||yo3_desktop::server_address(&self.draft.host_address).err()).flatten().map(|error|error.to_string());
        let host_invalid=host_problem.is_some() && (self.draft.host_address.contains(['/', '\\', '@']) || self.draft.host_address.trim().chars().any(char::is_whitespace));
        let port_invalid=(self.port!=self.settings.listen_port.to_string() && !self.port.is_empty()
            && !self.port.parse::<u16>().is_ok_and(|port|port>0)) || (self.port.is_empty() && self.failed && self.notice.starts_with("Port must be between"));
        let mut body=column![ui::label_help("Sharing address", "The LAN/VPN address or public hostname friends use to join. Router port forwarding can make this different from the network interface and port below."),
            text_input("LAN/VPN address or public hostname", &self.draft.host_address).padding(12)
                .style(move |theme,status|ui::input_style(theme,status,host_invalid))
                .on_input_maybe((!self.locked() && !self.modal_open()).then_some(|s|Msg::Input(Field::Host,s)))].spacing(16);
        if let Some(problem)=host_problem {body=body.push(ui::inline_status(problem,if host_invalid{ui::Tone::Danger}else{ui::Tone::Warning}));}
        let port=column![muted("TCP port"),text_input("7443",&self.port).padding(12)
            .style(move |theme,status|ui::input_style(theme,status,port_invalid))
            .on_input_maybe((!self.locked() && !self.modal_open()).then_some(|s|Msg::Input(Field::Port,s)))].spacing(8).width(150);
        let mut network=column![ui::label_help("Listen on", "The network interface the server accepts connections on. All interfaces (0.0.0.0) is the default. A specific interface limits access to that network; localhost allows only this computer."),muted("Keep All interfaces for hosting. Friends use the sharing address above, rather than 0.0.0.0."),
            pick_list(self.bindings.clone(),selected,Msg::BindSelect).placeholder("Choose interface").padding(12).width(Fill),
            row![port,widget::Space::new().width(Fill),action(Icon::Refresh,"Refresh interfaces",(!self.locked()).then_some(Msg::RefreshBindings)).style(button::text)].spacing(12).align_y(iced::Alignment::End)].spacing(16);
        if port_invalid {network=network.push(ui::inline_status("TCP port: use a number from 1 to 65535.",ui::Tone::Danger));}
        if self.status.is_some(){network=network.push(ui::inline_status("Stop the running server before saving hosting changes. Start it again to apply them.",ui::Tone::Info));}
        if let Some(listen)=self.status.as_ref().and_then(|s|s["listen"].as_str()){network=network.push(muted(format!("Active listener: {listen}")));}
        body=body.push(widget::rule::horizontal(1)).push(network).push(widget::rule::horizontal(1)).push(column![ui::label_help("Server data", "Accounts, cards and decks live in this folder. Leave it empty to use private application storage. Changing it selects a different server; it does not move existing data."),
            row![self.field("Data folder","Empty uses private application storage",&self.draft.data_dir,Field::Data),action(Icon::Folder,"Browse…",(!self.locked()).then_some(Msg::Browse(Field::Data)))].spacing(12).align_y(iced::Alignment::End),
            self.field("First player on a new server","Duelist",&self.draft.player_name,Field::Player),muted("Existing servers keep their accounts. Back up the complete data folder securely.")].spacing(16));
        body=body.push(widget::rule::horizontal(1)).push(self.server_options_view());
        body.into()
    }
    fn server_options_view(&self) -> Element<'_, Msg> {
        let query=self.option_filter.to_lowercase();
        let mut search=row![text_input("Find a flag…",&self.option_filter).padding(12).on_input(Msg::FindOption).width(Fill)].spacing(8).align_y(iced::Alignment::Center);
        if !self.option_filter.is_empty() {search=search.push(action(Icon::Close,"Clear filter",Some(Msg::FindOption(String::new()))).style(button::text));}
        let mut body=column![text("All server flags").size(20).font(bold()),
            muted("Generated from this server's CLI. Empty fields use defaults. Save hosting changes, then start the server."),
            search].spacing(16);
        let option_problem=if self.draft.server_options!=self.settings.server_options {
            yo3_desktop::server_options::arguments(&self.draft).err().map(|error|error.to_string())
        }else{None};
        let options=yo3_server::cli::server_options().into_iter().filter(|o| format!("{} {}",o.name,o.help).to_lowercase().contains(&query)).collect::<Vec<_>>();
        if let Some(problem)=&option_problem && !options.iter().any(|option|problem.contains(&format!("--{}",option.name))) {
            body=body.push(ui::inline_status(problem.lines().next().unwrap_or(problem).to_string(),
                if self.failed && self.notice.contains(problem.lines().next().unwrap_or(problem)){ui::Tone::Danger}else{ui::Tone::Warning}));
        }
        if options.is_empty() {body=body.push(muted("No server flags match this filter. Try another search or clear the filter."));}
        for option in options {
            let name=option.name.clone();
            let help=widget::tooltip(container(text("?").size(12)).padding(5).style(container::rounded_box),
                container(text(option.help).size(13)).padding(12).max_width(420).style(container::rounded_box),widget::tooltip::Position::Top).snap_within_viewport(true);
            let mut line=row![text(format!("--{name}")).font(iced::Font::MONOSPACE).size(14).width(210),help].spacing(10).align_y(iced::Alignment::Center);
            if let Some(owner)=yo3_desktop::server_options::managed(&name) { line=line.push(muted(owner)); }
            else {
                let value=self.draft.server_options.get(&name);
                if option.toggle {
                    let default=match name.as_str() {"no-packet-log"=>true,_=>option.default=="true"};
                    let selected=value.map_or(default,|v|v=="true");
                    let key=name.clone(); line=line.push(checkbox(selected).label(if selected{"On"}else{"Off"})
                        .on_toggle_maybe((!self.locked()).then_some(move |on:bool|Msg::ServerOption(key.clone(),on.to_string()))).width(Fill));
                } else {
                    let default=if name=="http-port"{"0 (automatic)"}else if option.default.is_empty(){"Optional"}else{&option.default};
                    let invalid=option_problem.as_ref().is_some_and(|problem|problem.contains(&format!("--{name}")))
                        && self.failed && self.notice.contains(&format!("--{name}"));
                    let key=name.clone();line=line.push(text_input(default,value.map_or("",String::as_str)).padding(10).width(Fill)
                        .style(move |theme,status|ui::input_style(theme,status,invalid))
                        .on_input_maybe((!self.locked()).then_some(move |s|Msg::ServerOption(key.clone(),s))));
                }
                if value.is_some() {line=line.push(action(Icon::Refresh,"Use default",(!self.locked()).then_some(Msg::ResetOption(name.clone()))).style(button::text));}
            }
            if let Some(problem)=&option_problem && problem.contains(&format!("--{name}")) {
                body=body.push(column![line,ui::inline_status(problem.lines().next().unwrap_or(problem).to_string(),
                    if self.failed && self.notice.contains(&format!("--{name}")){ui::Tone::Danger}else{ui::Tone::Warning})].spacing(8));
            } else {body=body.push(line);}
        }
        body.into()
    }
    fn accounts_view(&self)->Element<'_,Msg>{
        if self.status.is_none(){return column![text("Start your server to manage players").size(20),
            row![muted("Start with your saved hosting settings, then create one account for each friend and export their invitation."),action(Icon::Play,"Start server",(!self.locked()).then_some(Msg::Start)).style(button::primary)].spacing(16).align_y(iced::Alignment::Center)].spacing(16).into()}
        if self.account_form {
            let name=self.new_account.trim();let valid=!name.is_empty() && name.chars().count()<=yo3_server::accounts::MAX_NAME && !name.chars().any(char::is_control);
            let mut form=column![text("New account").size(20).font(bold()),muted("Create a separate account for each person you invite. Their first player will use this name."),
                self.field("First player's name","1–16 characters",&self.new_account,Field::Account),
                row![widget::Space::new().width(Fill),action(Icon::Close,"Cancel",(!self.locked()).then_some(Msg::NewAccount)).style(button::text),
                    action(Icon::Check,"Create account",(!self.locked() && valid).then_some(Msg::CreateAccount)).style(button::primary)].spacing(12)].spacing(16);
            if self.new_account.trim().is_empty(){form=form.push(muted("Choose a player name to create the account. You can export its invitation next."));}
            return form.into()
        }
        let names=account_names(&self.accounts);let share_ready=yo3_desktop::network::share_address(&self.settings).ok().flatten().is_some();
        if names.is_empty(){return column![text("No accounts yet").size(20).font(bold()),
            muted("Create your first account, then export a private invitation. Every friend should have their own account."),
            row![widget::Space::new().width(Fill),action(Icon::Plus,"Create account",(!self.locked()).then_some(Msg::NewAccount)).style(button::primary)]].spacing(16).into()}
        let mut selected=column![row![ui::label_help("Account", "Manage this account’s players and invitation. This selection is separate from your account on Play."),widget::Space::new().width(Fill),action(Icon::Plus,"New account",(!self.locked()).then_some(Msg::NewAccount)).style(button::text)].align_y(iced::Alignment::Center),pick_list(names,(!self.admin_account.is_empty()).then_some(self.admin_account.clone()),Msg::AdminAccountSelect).placeholder("Choose account").width(Fill).padding(12),
            muted("This selection manages invitations and players. Your account on Play is unchanged.")].spacing(16);
        let account=self.accounts.as_array().and_then(|a|a.iter().find(|a|a["players"][0]["name"].as_str()==Some(&self.admin_account)));
        let player_count=account.and_then(|a|a["players"].as_array()).map_or(0,Vec::len);
        if let Some(account)=account{let players=account["players"].as_array().map(|p|p.iter().filter_map(|p|p["name"].as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();selected=selected.push(muted(format!("Players: {players} · {} BP",account["bp"])));}
        if share_ready{selected=selected.push(row![muted(if self.admin_account.is_empty(){"Choose an account to export its invitation."}else{"Send this private file only to the account's owner. They import it under Play → Add server."}),
            action(Icon::Upload,"Export invitation…",(!self.locked() && !self.admin_account.is_empty()).then_some(Msg::Export)).style(button::primary)].spacing(16).align_y(iced::Alignment::Center));}
        else{selected=selected.push(row![ui::inline_status("Set a reachable sharing address before exporting invitations.",ui::Tone::Info),
            action(Icon::Settings,"Configure sharing",Some(Msg::HostTab(HostTab::Advanced))).style(button::text)].spacing(12).align_y(iced::Alignment::Center));}
        let mut body=column![selected].spacing(24);
        body=body.push(disclosure("Additional players",self.account_options,Msg::Disclosure(Disclosure::Account)));
        if self.account_options{
            let name=self.new_player.trim();let valid=!name.is_empty() && name.chars().count()<=yo3_server::accounts::MAX_NAME && !name.chars().any(char::is_control);
            body=body.push(column![ui::inline_status(if player_count>=yo3_server::accounts::MAX_PLAYERS{"This account has all four player slots. Create another account for a new player."}else if self.admin_account.is_empty(){"Choose an account above before adding a player."}else{"Each account can hold up to four players. Enter a name, then choose the player inside the game."},if player_count>=yo3_server::accounts::MAX_PLAYERS{ui::Tone::Warning}else{ui::Tone::Neutral}),self.field("Player name","New player on the selected account",&self.new_player,Field::NewPlayer),
                row![widget::Space::new().width(Fill),action(Icon::Plus,"Add player",(!self.locked() && !self.admin_account.is_empty() && valid && player_count<yo3_server::accounts::MAX_PLAYERS).then_some(Msg::AddPlayer)).style(button::primary)]].spacing(16));
        }
        body.into()
    }
    fn activity_view(&self)->Element<'_,Msg>{
        let shown = self.shown_logs();
        let empty=if !self.filter.is_empty(){"No activity matches this filter. Try another search or clear the filter."}
            else if self.paused{"Activity updates are paused. Clear Pause updates to show the latest logs."}
            else if self.log_server{"No server activity yet. Start your server from Hosting, or play on This computer."}
            else{"No game activity yet. Choose a server under Play and launch the game to see startup details here."};
        let mut controls=row![text_input("Filter activity…",&self.filter).on_input(Msg::LogFilter).padding(12).width(Fill)].spacing(12).align_y(iced::Alignment::Center);
        if !self.filter.is_empty() {controls=controls.push(action(Icon::Close,"Clear filter",Some(Msg::LogFilter(String::new()))).style(button::text));}
        controls=controls.push(checkbox(self.paused).label("Pause updates").on_toggle(Msg::Pause)).push(action(Icon::Copy,"Copy",(!shown.is_empty()).then_some(Msg::CopyLogs)).style(button::text));
        let mut body=column![self.page_heading("Activity","View session logs or export a diagnostic report."),row![ui::tab("Server",Msg::Logs(true),self.log_server),ui::tab("Game & launcher",Msg::Logs(false),!self.log_server),widget::Space::new().width(Fill),action(Icon::Download,"Export logs…",(!self.locked()).then_some(Msg::ExportLogs)).style(button::primary)].spacing(8),
            controls,
            ui::inline_status(if self.paused{"Updates paused · recording continues. Clear Pause updates to resume."}
                else if !self.follow_logs{"Scrolled up · scroll to the bottom to follow new activity."}
                else{"Following new activity · scroll up to read earlier entries."},if self.paused{ui::Tone::Warning}else{ui::Tone::Info})].spacing(16).height(Fill);
        if self.locked(){body=body.push(ui::inline_status("Log export is available when the current operation finishes.",ui::Tone::Neutral));}
        body.push(ui::log_panel(if shown.is_empty(){empty.into()}else{shown})).into()
    }
    fn settings_view(&self) -> Element<'_, Msg> {
        let changed = self.changed(Section::Game);
        let editable=!self.locked() && !self.game_running && !self.modal_open();
        let field_error=self.game_field_error();let invalid=field_error.is_some();
        let mut game = column![row![ui::label_help("Game installation", "Choose the complete original Yu-Gi-Oh! ONLINE 3 installation, including yo3.exe, yo3.idx and package/. Choosing a folder checks and saves it automatically. Play checks required components again before opening the game."),
                widget::Space::new().width(Fill), self.setup_badge()].spacing(12).align_y(iced::Alignment::Center),
            row![column![muted("Game folder"),text_input("Folder containing yo3.exe", &self.draft.game_dir).padding(12)
                    .on_input_maybe(editable.then_some(|s|Msg::Input(Field::Game,s))).style(move |theme,status|ui::input_style(theme,status,invalid))].spacing(8).width(Fill),
                action(Icon::Folder, "Browse…", editable.then_some(Msg::Browse(Field::Game)))].spacing(12).align_y(iced::Alignment::End)].spacing(12);
        if let Some(error)=field_error {game=game.push(ui::inline_status(error,ui::Tone::Danger));}
        game=game.push(muted("Your folder is checked and saved automatically. You can keep editing while a check runs."))
            .push(disclosure("Advanced game options", self.advanced_game, Msg::Disclosure(Disclosure::Game)));
        if self.advanced_game && !cfg!(windows) {
            game = game.push(column![ui::label_help("Wine prefix", "Linux only: a separate Wine environment containing the game's registry and Windows files. Leave empty to use your default Wine prefix."),
                text_input("Empty uses the default prefix", &self.draft.wine_prefix).padding(12)
                    .on_input_maybe(editable.then_some(|s| Msg::Input(Field::Wine, s)))].spacing(8));
        }
        let detail = if self.draft.game_dir.trim().is_empty() {"Choose the folder containing your original game to continue."}
        else if self.setup_in_progress() {"Checking game files and required connection components. This folder will be saved when the check succeeds…"}
        else {match &self.setup_result {
            None => "The game folder has not been checked yet.",
            Some(yo3_desktop::Readiness::Ready) => "Game verified and saved. Play checks required components again before opening the game.",
            Some(yo3_desktop::Readiness::GameProblem(_)) => "Check the highlighted game folder, then retry the check.",
            Some(yo3_desktop::Readiness::ComponentProblem(_)) => "Restore the required Revival files from the complete matching Desktop download, then retry.",
            Some(yo3_desktop::Readiness::ValidationProblem(_)) => "The check could not finish. Retry it; export logs if the problem continues.",
        }};
        let page_title=if invalid{heading("Game setup","Set up your game and optional direct launch.")}else{self.page_heading("Game setup","Set up your game and optional direct launch.")};
        let mut body=column![page_title,game].spacing(20);
        if invalid && self.feedback_visible(){body=body.push(self.page_heading("",""));}
        if !invalid {
            let tone=if self.setup_in_progress(){ui::Tone::Info}else{match &self.setup_result{Some(yo3_desktop::Readiness::Ready)=>ui::Tone::Success,Some(_)=>ui::Tone::Danger,None=>ui::Tone::Neutral}};
            body=body.push(ui::inline_status(detail,tone));
        }
        if !self.setup_in_progress() && self.setup_result.as_ref().is_some_and(|s| *s != yo3_desktop::Readiness::Ready) {
            let mut recovery=row![action(Icon::Refresh, "Retry check",(editable && !self.draft.game_dir.trim().is_empty()).then_some(Msg::ValidateGame)).style(button::primary)].spacing(12);
            if matches!(self.setup_result,Some(yo3_desktop::Readiness::ComponentProblem(_))) && !self.feedback_visible() {
                recovery=recovery.push(action(Icon::Download, "Download Desktop…", Some(Msg::ReleasePage)).style(button::text));
            }
            body=body.push(recovery);
        }
        if self.first_setup && !changed && !self.setup_in_progress() && matches!(self.setup_result, Some(yo3_desktop::Readiness::Ready)) {
            body = body.push(action(Icon::Play, "Continue to Play", Some(Msg::Page(Page::Play))).style(button::primary));
        }
        if self.advanced_game {
            let (label, tone) = match self.direct_state {
                yo3_desktop::direct::State::Disabled => ("Disabled", ui::Tone::Neutral),
                yo3_desktop::direct::State::Enabled => ("Enabled", ui::Tone::Success),
                yo3_desktop::direct::State::NeedsRepair(_) => ("Needs repair", ui::Tone::Warning),
                yo3_desktop::direct::State::Conflict(_) => ("Mod conflict", ui::Tone::Danger),
            };
            let mut direct = column![row![ui::label_help("Direct launch", "Optional setup for opening yo3.exe or its original shortcut without this window. A small reversible loader checks the saved server and connection DLL before login. The original executable and INI are unchanged."),
                    widget::Space::new().width(Fill), ui::status_badge(label.into(), tone)].spacing(12).align_y(iced::Alignment::Center),
                checkbox(self.direct_checked()).label("Allow opening yo3.exe directly")
                    .on_toggle_maybe(self.can_change_direct(!self.direct_checked()).then_some(Msg::Direct)),
                muted("Use your original game shortcut with the saved server and account. Start the server first when playing locally.")].spacing(12);
            match &self.direct_state {
                yo3_desktop::direct::State::Disabled | yo3_desktop::direct::State::Enabled => {},
                yo3_desktop::direct::State::NeedsRepair(reason) => direct = direct.push(ui::inline_status(reason,ui::Tone::Warning))
                    .push(muted("Repair restores Revival's loader (winmm.dll) beside yo3.exe and refreshes its saved server and account."))
                    .push(action(Icon::Refresh, "Repair direct launch", self.can_change_direct(true).then_some(Msg::Direct(true)))),
                yo3_desktop::direct::State::Conflict(reason) => direct = direct.push(ui::inline_status(reason,ui::Tone::Danger)),
            }
            if self.game_running {direct=direct.push(ui::inline_status("Close the game before changing direct launch.",ui::Tone::Info));}
            else if changed || self.setup_in_progress() { direct = direct.push(ui::inline_status("Wait for the current game folder to pass its check before changing direct launch.",if self.setup_in_progress(){ui::Tone::Info}else{ui::Tone::Warning})); }
            else if matches!(self.direct_state,yo3_desktop::direct::State::Disabled | yo3_desktop::direct::State::NeedsRepair(_)) && !matches!(self.setup_result,Some(yo3_desktop::Readiness::Ready)) {
                direct=direct.push(ui::inline_status(if self.direct_checked(){"Fix game setup above before repairing direct launch. You can still turn it off."}else{"Fix game setup above before enabling direct launch."},ui::Tone::Warning));
            }
            else if matches!(self.direct_state,yo3_desktop::direct::State::Disabled | yo3_desktop::direct::State::NeedsRepair(_))
                && self.selected==ServerChoice::Local && self.status.is_none() && !self.locked() {
                direct=direct.push(ui::inline_status("Start your server in Hosting before enabling or repairing direct launch.",ui::Tone::Info));
            }
            body = body.push(widget::rule::horizontal(1)).push(direct);
        }
        page_scroll(body)
    }
    fn about_view(&self)->Element<'_,Msg>{
        let (title,detail)=match &self.update_status{
            UpdateStatus::Checking=>("Checking for updates…".into(),"Looking for a published release on GitHub.".into()),UpdateStatus::Failed(e)=>("Couldn't check for updates".into(),e.clone()),
            UpdateStatus::Checked(updates::Decision::Available(r))=>(format!("Version {} is available",r.tag),"Review the release before downloading and restarting.".into()),
            UpdateStatus::Checked(updates::Decision::Current{latest})=>("You're up to date".into(),format!("Latest published release: {latest}.")),
            UpdateStatus::Checked(updates::Decision::Development{latest})=>("Development build".into(),format!("Latest published release: {latest}. This build stays in place.")),
            UpdateStatus::Checked(updates::Decision::Manual{latest})=>(format!("Version {latest} is available"),"See GitHub for a compatible build.".into()),
            UpdateStatus::Checked(updates::Decision::NoReleases)=>("No published releases yet".into(),"Check again later.".into()),
        };
        let tone=match &self.update_status {
            UpdateStatus::Checking|UpdateStatus::Checked(updates::Decision::Available(_))=>ui::Tone::Info,
            UpdateStatus::Failed(_)=>ui::Tone::Danger,
            UpdateStatus::Checked(updates::Decision::Current{..})=>ui::Tone::Success,
            UpdateStatus::Checked(updates::Decision::Manual{..})=>ui::Tone::Warning,
            _=>ui::Tone::Neutral,
        };
        let title=text(title).size(20).font(bold()).color(ui::tone_color(tone));
        let mut update = column![if matches!(self.update_status,UpdateStatus::Checking){row![self.spinner(),title].spacing(10)}else{row![title]},ui::inline_status(detail,tone)].spacing(16);
        if matches!(self.update_status,UpdateStatus::Failed(_)) {
            update=update.push(muted("You can still play. Retry the update check or open the release page manually."));
        }
        match &self.update_status {
            UpdateStatus::Checking=>{},
            UpdateStatus::Checked(updates::Decision::Available(r))=>{
                update=update.push(row![muted(format!("Download size: {}",size(r.asset.bytes))),
                    action(Icon::Download,"Review & install…",(!self.locked()).then_some(Msg::ReviewUpdate)).style(button::primary)].spacing(12).align_y(iced::Alignment::Center))
                    .push(row![text("Release notes").size(17).font(bold()).width(Fill),
                        action(Icon::External,"View on GitHub",Some(Msg::ReleasePage)).style(button::text)].spacing(12).align_y(iced::Alignment::Center))
                    .push(ui::release_notes_panel(&self.release_notes,260.0));
            }
            UpdateStatus::Checked(updates::Decision::Manual {..})=>{
                update=update.push(action(Icon::Download,"Open release downloads",Some(Msg::ReleasePage)).style(button::primary));
            }
            UpdateStatus::Failed(_)=>{
                update=update.push(row![action(Icon::Refresh,"Try again",(!self.locked()).then_some(Msg::CheckUpdates)).style(button::primary),
                    action(Icon::External,"Open releases",Some(Msg::ReleasePage)).style(button::text)].spacing(12));
            }
            _=>update=update.push(action(Icon::Refresh,"Check for updates",(!self.locked()).then_some(Msg::CheckUpdates)).style(button::primary)),
        }
        if self.locked() && !matches!(self.update_status,UpdateStatus::Checking){update=update.push(ui::inline_status("Finish the current operation before checking or installing an update.",ui::Tone::Neutral));}
        let mut body = column![self.page_heading("About & updates", "Stay current with YO3 Revival."),
            column![row![text(format!("YO3 Revival {}", yo3_tunnel::BUILD_VERSION)).size(23).font(bold()).width(Fill),
                action(Icon::Copy, "Copy version", Some(Msg::CopyVersion)).style(button::text)].spacing(12),
                muted(format!("Build {} · {}", yo3_tunnel::BUILD_COMMIT, std::env::consts::OS))].spacing(12),
            row![action(Icon::External,"GitHub",Some(Msg::GitHub)).style(button::text),
                action(Icon::Folder,"Open application data",Some(Msg::OpenData)).style(button::text)].spacing(12),
            widget::rule::horizontal(1),update].spacing(20);
        if !self.settings.advanced_mode { body=column![action(Icon::Play,"Back to game",Some(Msg::Page(Page::Play))).style(button::text)].push(body).spacing(18); }
        body=body.push(muted("Update checks run automatically when you open Revival. Download and installation always require your confirmation; game progress stays in place."));page_scroll(body)
    }
    fn setup_badge(&self)->Element<'_,Msg>{
        let (label,tone)=if self.setup_in_progress(){("Checking…",ui::Tone::Info)}else if self.draft.game_dir.trim().is_empty(){("Choose game folder",ui::Tone::Neutral)}else{match &self.setup_result{None=>("Not checked yet",ui::Tone::Neutral),Some(yo3_desktop::Readiness::Ready)=>("Ready to play",ui::Tone::Success),Some(yo3_desktop::Readiness::GameProblem(_))=>("Game folder needs attention",ui::Tone::Danger),Some(yo3_desktop::Readiness::ComponentProblem(_))=>("Revival files need repair",ui::Tone::Danger),Some(yo3_desktop::Readiness::ValidationProblem(_))=>("Check failed",ui::Tone::Danger)}};
        ui::status_badge(label.into(),tone)
    }
    fn save_actions(&self,section:Section)->Element<'_,Msg>{
        let changed=self.changed(section);let running=section==Section::Hosting && self.status.is_some();
        if !changed {return widget::Space::new().height(0).into()}
        row![muted(if running{"Stop your server before saving these changes."}else{"Unsaved changes"}),
            action(Icon::Close,"Cancel",(!self.locked()).then_some(Msg::CancelSettings(section))).style(button::text),
            action(Icon::Check,"Save changes",(!self.locked() && !running).then_some(Msg::SaveSettings(section))).style(button::primary)].spacing(12).align_y(iced::Alignment::Center).into()
    }
    fn field<'a>(&self,label:&'static str,hint:&'static str,value:&str,field:Field)->Element<'a,Msg>{
        let problem=match field {
            Field::Address if !value.trim().is_empty()=>yo3_desktop::server_address(value).err().map(|error|{
                let error=error.to_string();let definite=value.contains(['/', '\\', '@']) || value.trim().chars().any(char::is_whitespace)
                    || (self.failed && self.notice.contains(&error));
                (error,if definite{ui::Tone::Danger}else{ui::Tone::Warning})
            }),
            Field::Player|Field::Account|Field::NewPlayer if value.trim().chars().count()>yo3_server::accounts::MAX_NAME=>
                Some((format!("Use at most {} characters for a player name.",yo3_server::accounts::MAX_NAME),ui::Tone::Warning)),
            Field::Player|Field::Account|Field::NewPlayer if value.chars().any(char::is_control)=>
                Some(("Use ordinary text without line breaks or control characters for a player name.".into(),ui::Tone::Warning)),
            _=>None,
        };
        let invalid=problem.as_ref().is_some_and(|(_,tone)|matches!(tone,ui::Tone::Danger));
        let mut body=column![muted(label),text_input(hint,value).padding(12).style(move |theme,status|ui::input_style(theme,status,invalid))
            .on_input_maybe((!self.locked() && !self.modal_open()).then_some(move |v|Msg::Input(field,v)))].spacing(8).width(Fill);
        if let Some((message,tone))=problem {body=body.push(ui::inline_status(message,tone));}
        body.into()
    }
}
pub(super) fn display_path(path:&std::path::Path)->String {
    let path=path.to_string_lossy();
    if let Some(unc)=path.strip_prefix(r"\\?\UNC\") {format!(r"\\{unc}")}
    else {path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()}
}
fn bold()->iced::Font{iced::Font{weight:iced::font::Weight::Semibold,..iced::Font::with_name("Fira Sans")}}
fn heading<'a>(title:&'static str,description:&'static str)->Element<'a,Msg>{column![text(title).size(28).font(bold()),muted(description)].spacing(8).into()}
fn metric<'a>(label:&'static str,value:u64)->Element<'a,Msg>{column![text(value.to_string()).size(26),muted(label)].spacing(4).into()}
fn page_scroll(body:widget::Column<'_,Msg>)->Element<'_,Msg>{scrollable(body.width(Fill)).spacing(16).width(Fill).height(Fill).into()}
pub(super) fn size(bytes:u64)->String{if bytes<1_000_000{format!("{:.1} KB",bytes as f64/1000.0)}else{format!("{:.1} MB",bytes as f64/1_000_000.0)}}

pub(super) fn friendly_logs(logs: &str) -> String {
    let mut previous=0;
    let mut lines=logs.lines().map(|line| {
        if let Some((stamp,message))=line.split_once(' ') && let Ok(stamp)=stamp.parse::<i64>() {
            previous=stamp; return (stamp,message.to_string(),true)
        }
        (previous,line.to_string(),false)
    }).collect::<Vec<_>>();
    lines.sort_by_key(|(stamp,_,_)|*stamp);
    let mut groups:Vec<(i64,String,bool,usize)>=Vec::new();let mut hidden=0;
    for (stamp,message,dated) in lines {
        // Server tracing has its own ISO timestamp inside the session timestamp. Show one readable local time.
        let message=if dated && let Some((iso,rest))=message.split_once(' ') {
            if chrono::DateTime::parse_from_rfc3339(iso).is_ok() { rest.trim_start().to_string() } else { message }
        } else { message };
        if message.trim().is_empty(){continue}
        let wine=message.strip_prefix("Game helper: ").unwrap_or(&message);
        if let Some((thread,detail))=wine.split_once(':')
            && (4..=8).contains(&thread.len()) && thread.bytes().all(|c|c.is_ascii_hexdigit()) && detail.starts_with("fixme:") {
            hidden+=1;continue
        }
        if let Some(last)=groups.last_mut() && last.1==message {
            last.0=stamp;last.2|=dated;last.3+=1;
        } else {groups.push((stamp,message,dated,1));}
    }
    let mut shown=groups.into_iter().map(|(stamp,mut message,dated,count)| {
        if count>1{message.push_str(&format!(" (×{count})"));}
        if !dated{return message}
        let Some(date)=chrono::DateTime::from_timestamp(stamp,0) else{return format!("{stamp} {message}")};
        format!("{} {message}",date.with_timezone(&chrono::Local).format("%H:%M:%S"))
    }).collect::<Vec<_>>();
    if hidden>0 {shown.push(format!("{hidden} Wine diagnostic line{} hidden; full output is kept in exported logs.",if hidden==1{""}else{"s"}));}
    shown.join("\n")
}
