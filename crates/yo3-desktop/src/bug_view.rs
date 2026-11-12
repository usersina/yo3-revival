use super::*;
use bug_ui::BugEvent;
use iced::widget::{column,row};
use ui::{Icon,Tone,action,muted};

fn bold()->iced::Font {iced::Font {weight:iced::font::Weight::Semibold,..iced::Font::with_name("Fira Sans")}}

impl App {
    pub(super) fn bug_report_view<'a>(&'a self, base:Element<'a,Msg>) -> Element<'a, Msg> {
        let Some(bug)=&self.bug_report else{return base};
        let busy=self.report_busy || bug.auth_busy;
        let frozen=bug.frozen();
        let completed=bug.receipt.as_ref().is_some_and(|receipt|receipt.issue_url.is_some());
        let pending=bug.receipt.as_ref().is_some_and(|receipt|receipt.pending.is_some());
        let event=|event|Msg::Bug(event);
        let mut body=column![text(if completed{"Bug report published"}else if frozen{"Resume your bug report"}else{"Report a bug"}).size(24).font(bold()),
            muted(if frozen{"Your saved report is kept intact while its posting status is checked."}
                else{"Describe the problem. Your version, system details and diagnostic logs are included."})].spacing(12);

        if let Some(device)=&bug.device {
            let mut sign_in=column![text("Sign in to GitHub").size(24).font(bold()),
                muted("Enter this code at github.com/login/device, then approve sign-in."),
                row![text(&device.user_code).size(28).font(iced::Font::MONOSPACE).width(Fill),
                    action(Icon::Copy,"Copy code",Some(event(BugEvent::CopyCode))).style(button::text)].spacing(10).align_y(iced::Alignment::Center),
                row![action(Icon::External,"Open GitHub",Some(event(BugEvent::OpenSignIn))).style(button::text),
                    action(Icon::Close,"Cancel sign-in",Some(event(BugEvent::StopSignIn))).style(button::text)].spacing(12),
                row![self.spinner(),muted("Waiting for you to approve sign-in in your browser…")].spacing(10),
                muted("Signing in does not publish your report. You will choose Publish after returning here.")].spacing(16);
            if let Some(warning)=&bug.auth_warning {sign_in=sign_in.push(ui::inline_status(warning,Tone::Warning));}
            if let Some(Err(error))=&self.report_feedback {sign_in=sign_in.push(ui::notice(ui::inline_status(error,Tone::Danger),Tone::Danger));}
            let footer=row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)).style(button::text),widget::Space::new().width(Fill),
                action(Icon::External,"Waiting for sign-in…",None).width(220).style(button::primary)].width(Fill).spacing(12);
            return ui::modal_actions(base,sign_in.into(),footer.into())
        }

        if frozen {
            let payload=bug.receipt.as_ref().and_then(|receipt|receipt.payload.as_ref())
                .and_then(|value|serde_json::from_value::<yo3_desktop::bug_publish::Payload>(value.clone()).ok());
            if let Some(payload)=payload {
                body=body.push(text(payload.title).size(17).font(bold()))
                    .push(ui::disclosure("Review saved report",bug.details,event(BugEvent::Details(!bug.details))));
                if bug.details {body=body.push(text(payload.content).size(13).width(Fill).wrapping(widget::text::Wrapping::WordOrGlyph));}
                else {
                    let excerpt=payload.content.split("## What happened?\n\n").nth(1).unwrap_or(&payload.content)
                        .split("\n\n## ").next().unwrap_or(&payload.content);
                    let mut preview=excerpt.chars().take(220).collect::<String>();if excerpt.chars().count()>220 {preview.push('…');}
                    body=body.push(muted(preview));
                }
            } else {body=body.push(ui::inline_status("The saved report details could not be read. Check its status or export logs for help.",Tone::Warning));}
            if let Some(receipt)=&bug.receipt {
                if let Some(owner)=&receipt.account_login {body=body.push(muted(format!("Report account: {owner}")));}
                if receipt.gist_url.is_some() && !completed {
                    body=body.push(action(Icon::External,"View uploaded logs",Some(event(BugEvent::OpenGist))).style(button::text));
                }
            }
        } else {
            let mut description=widget::text_editor(&bug.description)
                .placeholder(if bug.context.error.trim().is_empty(){"What were you doing, and what went wrong?"}else{"Add any details that help explain the captured error (optional)."})
                .height(82).padding(10);
            if !self.report_busy {description=description.on_action(|action|Msg::Bug(BugEvent::Description(action)));}
            body=body.push(column![text("What happened?").size(15).font(bold()),
                description].spacing(6));
            if !bug.context.error.trim().is_empty() {
                let error=yo3_desktop::reports::filter_text(&bug.context.error,self.report_remove_personal);
                let summary=error.lines().find(|line|!line.trim().is_empty()).unwrap_or("An error was captured.").chars().take(180).collect::<String>();
                body=body.push(ui::inline_status(format!("Captured error: {summary}"),Tone::Danger));
            }
            body=body.push(row![ui::disclosure("Steps to reproduce",bug.details,event(BugEvent::Details(!bug.details))),
                ui::disclosure("Diagnostic options",bug.log_options,event(BugEvent::LogOptions(!bug.log_options)))].spacing(8));
            if bug.details {
                let mut steps=widget::text_editor(&bug.steps).placeholder("1. Open…\n2. Choose…\n3. The problem happens…").height(86).padding(10);
                if !self.report_busy {steps=steps.on_action(|action|Msg::Bug(BugEvent::Steps(action)));}
                body=body.push(steps);
            }
            if let Some(system)=&bug.system {
                let mut label=format!("{} · {} · {}",yo3_tunnel::BUILD_VERSION,system.os,system.arch);
                if let Some(wine)=&system.wine {label.push_str(&format!(" · {wine}"));}
                body=body.push(muted(label));
            }
            if let Some(report)=&self.report {
                body=body.push(muted(format!("{} · {} diagnostic files · {}",if self.report_all{"All retained sessions"}else{"Current session"},report.preview.files,views::size(report.preview.bytes))));
            } else if !self.report_busy {
                body=body.push(muted("No session logs available. System details and the captured error will still be included."));
            }
            if bug.log_options {
                if !self.report_busy {
                    body=body.push(widget::radio("Current session",false,Some(self.report_all),Msg::PrepareReport))
                        .push(widget::radio("All retained sessions",true,Some(self.report_all),Msg::PrepareReport));
                }
                body=body.push(muted("Only logs retained by Revival are included. Game files, databases and credentials are excluded."));
            }
            body=body.push(ui::report_filter(self.report_remove_personal,!self.report_busy));
            if self.report_remove_personal {
                body=body.push(muted("Detailed game traces are also omitted; original logs stay unchanged."));
            }
        }

        if let Some(warning)=&bug.warning {body=body.push(ui::inline_status(warning,Tone::Warning));}
        if !bug.recovery_checked && !self.report_busy {
            body=body.push(action(Icon::Refresh,"Retry preview",Some(event(BugEvent::RetryPreview))).style(button::text));
        }
        if let Some(account)=&self.github_account {
            let mut account_row=row![ui::inline_status(format!("Signed in as {}",account.login),Tone::Success)].align_y(iced::Alignment::Center).spacing(10);
            if !busy {account_row=account_row.push(action(Icon::Close,"Sign out",Some(event(BugEvent::SignOut))).style(button::text));}
            body=body.push(account_row);
            if let Some(warning)=&account.warning {body=body.push(ui::inline_status(warning,Tone::Warning));}
        }
        if let Some(warning)=&bug.auth_warning {body=body.push(ui::inline_status(warning,Tone::Warning));}
        if self.report_busy {body=body.push(ui::inline_status(bug.phase,Tone::Info));}
        else if bug.auth_busy {body=body.push(muted(if self.github_account.is_some(){"Saving GitHub sign-in securely…"}else{"Checking GitHub sign-in…"}));}
        if let Some(result)=&self.report_feedback {
            let (message,tone)=match result {Ok(message)=>(message,Tone::Success),Err(message)=>(message,Tone::Danger)};
            body=body.push(ui::notice(ui::inline_status(message,tone),tone));
        }
        let has_error=bug.auth_warning.is_some() || self.report_feedback.as_ref().is_some_and(Result::is_err);
        if has_error && !self.report_busy {
            let mut recovery=row![action(Icon::Copy,"Copy report",Some(event(BugEvent::Copy))).style(button::text),
                action(Icon::Download,"Export logs…",Some(event(BugEvent::Export))).style(button::text)].spacing(8);
            if bug.receipt.as_ref().is_some_and(|receipt|receipt.issue_url.is_some() || receipt.gist_url.is_some()) {
                recovery=recovery.push(action(Icon::Copy,"Copy link",Some(event(BugEvent::CopyLink))).style(button::text));
            }
            body=body.push(recovery);
        }
        if !busy && !completed && bug.recovery_checked && bug.system.is_some() && !bug.can_submit() {
            body=body.push(muted("Describe what went wrong to continue."));
        } else if !busy && !completed && !bug.recovery_checked {
            body=body.push(muted("Retry preview to check saved report progress before continuing."));
        }
        let (label,icon,message)=if completed {
            ("View issue",Icon::External,(!busy).then_some(event(BugEvent::OpenIssue)))
        } else if busy {
            (if self.report_busy {if pending{"Checking status…"}else if frozen{"Publishing…"}else{"Preparing…"}}else if self.github_account.is_some(){"Saving sign-in…"}else{"Checking sign-in…"},Icon::Refresh,None)
        } else if self.github_account.is_none() {
            ("Sign in to GitHub",Icon::External,bug.can_submit().then_some(event(BugEvent::SignIn)))
        } else if pending {
            ("Check report status",Icon::Refresh,bug.can_submit().then_some(event(BugEvent::Publish(false))))
        } else {
            (if frozen{"Retry publishing"}else{"Publish bug report"},Icon::Upload,bug.can_submit().then_some(event(BugEvent::Publish(false))))
        };
        let primary:Element<'_,Msg>=if busy {
            button(row![self.spinner(),text(label).size(14)].spacing(9).align_y(iced::Alignment::Center))
                .padding([10,14]).width(220).style(button::primary).into()
        } else {action(icon,label,message).width(220).style(button::primary).into()};
        let actions=row![action(Icon::Close,if completed{"Close"}else{"Cancel"},Some(Msg::CancelClose)).style(button::text),
            widget::Space::new().width(Fill),primary].width(Fill).spacing(12).align_y(iced::Alignment::Center);
        let footer:Element<'_,Msg>=if completed {actions.into()}else{
            column![ui::inline_status("Public GitHub issue. Anyone with the link can also read its diagnostic logs.",Tone::Warning),
                muted(if !frozen && !self.report_remove_personal {"Logs may include names, chat, decks, paths and addresses. Credentials are removed."}
                    else{"Review the saved details before publishing. Credentials are always removed."}),actions].spacing(8).into()
        };
        ui::modal_actions(base,body.into(),footer)
    }
}
