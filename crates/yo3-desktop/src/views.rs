use super::*;
use iced::widget::{column, row};
use ui::{Icon, action, card, disclosure, muted};
impl App {
    pub(super) fn view(&self) -> Element<'_, Msg> {
        let mut nav = column![row![ui::icon(Icon::Layers).width(28).height(28).style(|_,_| widget::svg::Style { color: Some(ui::GOLD) }),
            column![text("YO3").size(26).font(bold()),text("REVIVAL").size(11).color(ui::MUTED)].spacing(1)].spacing(12).align_y(iced::Alignment::Center),widget::Space::new().height(20)].spacing(6);
        for (label,icon,page) in [("Play",Icon::Play,Page::Play),("My server",Icon::Server,Page::Server),("Activity",Icon::Activity,Page::Activity),("Settings",Icon::Settings,Page::Settings)] {
            let selected=self.page==page;
            nav=nav.push(action(icon,label,Some(Msg::Page(page))).width(Fill).style(move |theme,status| {
                let mut style=button::text(theme,status); if selected { style.background=Some(Color::from_rgb8(36,57,79).into()); } style.border.radius=8.0.into(); style
            }));
        }
        nav=nav.push(widget::Space::new().height(Fill)).push(action(Icon::Book,"Help",Some(Msg::Help)).width(Fill).style(button::text))
            .push(action(Icon::External,"GitHub",Some(Msg::GitHub)).width(Fill).style(button::text)).push(widget::rule::horizontal(1))
            .push(button(column![text(format!("Version {}",yo3_tunnel::BUILD_VERSION)).size(12),text(match self.update_status { UpdateStatus::Checked(updates::Decision::Available(_))=>"Update available", _=>"About & updates" }).size(11).color(ui::MUTED)].spacing(5)).on_press(Msg::Page(Page::About)).padding([10,4]).width(Fill).style(button::text));
        let content=match self.page { Page::Play=>self.play_view(),Page::Server=>self.server_view(),Page::Activity=>self.activity_view(),Page::Settings=>self.settings_view(),Page::About=>self.about_view() };
        let mut main=column![].spacing(16).padding(28).width(Fill).height(Fill);
        if let Some((release,transfer))=&self.transfer {
            let detail=if transfer.cancelled() { "Cancelling…".into() } else if transfer.verifying() { "Verifying and unpacking…".into() } else { format!("{} of {}",size(transfer.bytes()),size(release.asset.bytes)) };
            main=main.push(card(column![row![text(format!("Updating to {}",release.tag)).size(16).width(Fill),action(Icon::Close,"Cancel download",(!transfer.cancelled()).then_some(Msg::CancelUpdate)).style(button::text)].spacing(10),widget::progress_bar(0.0..=release.asset.bytes as f32,transfer.bytes() as f32).girth(5),muted(detail)].spacing(12)));
        } else if self.applying || self.staged.is_some() { main=main.push(card(column![text("Restarting into the update…").size(16),muted("Waiting for the game/server to close and the new launcher to become ready.")].spacing(8))); }
        let base=row![container(nav).width(Fixed(202.0)).height(Fill).padding(20).style(|_|container::Style{background:Some(Color::from_rgb8(19,26,37).into()),..Default::default()}),main.push(content)].into();
        if self.review_update && let UpdateStatus::Checked(updates::Decision::Available(release))=&self.update_status {
            return ui::modal(base,column![heading("Install this update?","The launcher, server and connection components update together."),text(format!("{} → {} · {}",yo3_tunnel::BUILD_VERSION,release.tag,size(release.asset.bytes))).size(18),
                scrollable(text(if release.notes.is_empty(){"No release notes provided."}else{&release.notes}).size(14)).height(Fixed(170.0)),
                muted("After verification, games started here will close. Close separately opened games first. Your hosted server will restart and disconnect its players. Saved settings, accounts and decks stay in place."),
                muted("The current version remains available if the new launcher cannot open. Existing shortcuts will open the updated version."),
                row![action(Icon::Close,"Later",Some(Msg::CancelClose)),action(Icon::Download,"Download & restart",Some(Msg::ConfirmUpdate)).style(button::primary)].spacing(12)].spacing(20).into());
        }
        if self.report_options {
            let mut body=column![heading("Export logs","Choose which retained sessions to include."),
                widget::radio("Current session",false,Some(self.report_all),Msg::PrepareReport),widget::radio("All retained sessions",true,Some(self.report_all),Msg::PrepareReport),
                muted("Sessions are retained for up to 14 days or 100 MiB. The export includes logs and version information; credentials, databases and game files are excluded.")].spacing(16);
            if let Some(report)=&self.report {
                let p=&report.preview; let date=|n|chrono::DateTime::from_timestamp(n as i64,0).map(|d|d.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(||"No sessions".into());
                body=body.push(text(format!("{} files · {} before compression",p.files,size(p.bytes))).size(17)).push(muted(format!("{} to {}",date(p.oldest),date(p.newest))));
            } else { body=body.push(muted(if self.busy{"Preparing preview…"}else{"No preview available. Select a scope to retry."})); }
            body=body.push(muted("Detailed tracing can contain player/chat information. Review the ZIP before sharing. Nothing is uploaded."))
                .push(row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)),action(Icon::Download,"Save ZIP…",(!self.busy && self.report.is_some()).then_some(Msg::SaveLogs)).style(button::primary)].spacing(12));
            return ui::modal(base,body.into());
        }
        if let Some(observed)=&self.observed {
            let compatible=observed.info.ensure_compatible(yo3_tunnel::PROTOCOL_VERSION,yo3_tunnel::MOD_ABI);
            return ui::modal(base,column![heading("Confirm this server","Compare this identity with the host before connecting."),text(&self.address).size(18),
                ui::badge(format!("{} · {}",observed.info.release,if compatible.is_ok(){"Compatible"}else{"Update required"}),compatible.is_ok()),
                muted("Ask the host to confirm this code. An imported invitation already carries the expected identity."),container(text(&observed.identity).font(iced::Font::MONOSPACE).size(12)).padding(14).style(container::rounded_box),
                if let Err(error)=&compatible{text(error.to_string()).color(ui::BAD)}else{text("Your account key has not been sent.").size(13).color(ui::MUTED)},
                row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)),action(Icon::Shield,"Trust this server",compatible.is_ok().then_some(Msg::TrustIdentity)).style(button::primary)].spacing(12)].spacing(20).into());
        }
        if self.remove_server.is_some() {
            return ui::modal(base,column![heading("Remove this connection?","This removes its saved address and credentials from this launcher."),muted("The account and its game data remain on the server."),row![action(Icon::Close,"Cancel",Some(Msg::CancelClose)),action(Icon::Close,"Remove connection",Some(Msg::ConfirmRemoveServer)).style(button::danger)].spacing(12)].spacing(20).into());
        }
        if self.repair_setup {
            let mut body=column![heading("Repair game setup","Use a complete game installation and matching application components.")].spacing(16);
            match &self.setup_result {
                Some(yo3_desktop::Readiness::GameProblem(error))=>{body=body.push(text(error).size(14)).push(muted("Choose the complete original installation, then save the Game section. The launcher does not download original game files."))
                    .push(action(Icon::Folder,"Choose game folder…",Some(Msg::Browse(Field::Game))).style(button::primary));}
                Some(yo3_desktop::Readiness::ComponentProblem(error))=>{body=body.push(text(error).size(14)).push(muted("Restore the complete application bundle from a release download and open its launcher. Your game installation and saved application data stay in place. Avoid mixing individual DLLs from different releases."))
                    .push(action(Icon::Download,"Restore application files…",Some(Msg::ReleasePage)).style(button::primary));}
                _=>body=body.push(muted("Check the setup again to refresh component status.")),
            }
            return ui::modal(base,body.push(row![action(Icon::Close,"Close",Some(Msg::CancelClose)),action(Icon::Refresh,"Check again",Some(Msg::CheckSetup))].spacing(12)).into());
        }
        if self.closing {
            let mut body=column![heading("Close YO3 Revival?","A game started here will close.")].spacing(16);
            if self.status.is_some() {body=body.push(action(Icon::Server,"Keep hosting & exit",(!self.locked()).then_some(Msg::Quit(false))).width(Fill).style(button::primary))
                .push(action(Icon::Stop,"Stop server & exit",(!self.locked()).then_some(Msg::Quit(true))).width(Fill).style(button::danger));}
            else{body=body.push(action(Icon::Close,"Close launcher",(!self.locked()).then_some(Msg::Quit(false))).width(Fill).style(button::primary));}
            return ui::modal(base,body.push(action(Icon::Close,"Keep open",Some(Msg::CancelClose)).width(Fill).style(button::text)).into());
        }
        base
    }
    fn page_heading(&self,title:&'static str,description:&'static str)->Element<'_,Msg>{
        let mut body=column![heading(title,description)].spacing(16);
        if self.busy && self.operation_page==self.page {body=body.push(container(row![ui::icon(Icon::Refresh),muted(&self.operation_label)].spacing(10)).padding(12).width(Fill).style(container::rounded_box));}
        else if !self.notice.is_empty() && self.feedback_page==self.page {
            body=body.push(container(row![ui::icon(if self.failed{Icon::Close}else{Icon::Check}),text(&self.notice).size(13).color(if self.failed{ui::BAD}else{ui::GOOD}).width(Fill),
                button(ui::icon(Icon::Close)).padding(4).style(button::text).on_press(Msg::DismissNotice)].spacing(10).align_y(iced::Alignment::Center)).padding(12).width(Fill).style(container::rounded_box));
        }
        body.into()
    }
    fn play_view(&self) -> Element<'_, Msg> {
        let mut body = column![self.page_heading("Play", "Choose a server, then launch your game.")].spacing(28);
        if let UpdateStatus::Checked(updates::Decision::Available(release)) = &self.update_status {
            body = body.push(row![muted(format!("Version {} is available", release.tag)),
                action(Icon::Download, "View update", Some(Msg::Page(Page::About))).style(button::text)]
                .spacing(12).align_y(iced::Alignment::Center));
        }
        let editable = !self.locked() && !self.game_running;
        let mut servers = vec![ServerChoice::Local]; servers.extend(self.servers.iter().map(ServerChoice::saved));
        let mut connection = column![row![
            ui::label_help("Server", "My server runs on this computer. Add a server to join a host by address, or import the private invitation they sent you."),
            widget::Space::new().width(Fill), action(Icon::Plus, "Add server", (editable && !self.editor).then_some(Msg::AddServer)).style(button::text)]
            .align_y(iced::Alignment::Center)].spacing(14);
        if self.game_running {
            connection = connection.push(text(self.selected.to_string()).size(18));
        } else {
            connection = connection.push(pick_list(servers, Some(self.selected.clone()), Msg::Select).width(Fill).padding(12));
        }
        let mut actions = row![action(Icon::Upload, "Import invitation…", editable.then_some(Msg::Import)).style(button::text)].spacing(8);
        if self.selected == ServerChoice::Local {
            let names = account_names(&self.accounts);
            if !names.is_empty() {
                connection = connection.push(ui::label_help("Your account", "This is the account used to play. Each account can have several players; choose your player inside the game. Managing another account under My server does not change this selection."));
                if self.game_running { connection = connection.push(text(&self.account)); }
                else { connection = connection.push(pick_list(names, (!self.account.is_empty()).then_some(self.account.clone()), Msg::AccountSelect).width(Fill).padding(12)); }
            }
            connection = connection.push(muted("Your server starts automatically when you play."));
        } else {
            actions = actions.push(action(Icon::Shield, "Test connection", (!self.locked()).then_some(Msg::TestConnection)).style(button::text))
                .push(action(Icon::Settings, "Edit", editable.then_some(Msg::EditServer)).style(button::text))
                .push(action(Icon::Close, "Remove", editable.then_some(Msg::RemoveServer)).style(button::text));
        }
        connection = connection.push(actions);
        if self.game_running { connection = connection.push(muted("Close the game to change its server or account.")); }
        if !self.editor && let Some(info) = &self.checked {
            connection = connection.push(ui::badge(format!("Server {} · {}", info.release, if info.ready { "Ready" } else { "Rules engine unavailable" }), info.ready));
        }
        if self.editor { connection = connection.push(widget::rule::horizontal(1)).push(self.connection_editor()); }
        body = body.push(connection).push(widget::rule::horizontal(1));
        let mut launch = column![].spacing(12);
        let primary = if self.game_running {
            launch = launch.push(text(if self.game_ready { "Game running" } else { "Starting game…" }).size(21).font(bold()));
            action(Icon::Stop, "Close game", (!self.busy && !self.applying).then_some(Msg::StopGame)).style(button::danger)
        } else { action(Icon::Play, "Play", self.can_play().then_some(Msg::Play)).padding([14, 32]).style(button::primary) };
        launch = launch.push(row![self.setup_badge(), widget::Space::new().width(Fill), primary].spacing(12).align_y(iced::Alignment::Center));
        if !matches!(self.setup_result, Some(yo3_desktop::Readiness::Ready)) {
            launch = launch.push(action(Icon::Folder, "Set up game", Some(Msg::Page(Page::Settings))).style(button::text));
        }
        body = body.push(launch).push(muted("In the game: ONLINE → dismiss the login form → choose your player. Before your first duel, open Main Menu → Edit Deck once to save your starter deck."));
        page_scroll(body)
    }
    fn connection_editor(&self) -> Element<'_, Msg> {
        let mut body = column![row![text(if self.editing_id.is_some() { "Edit connection" } else { "New connection" }).size(18).font(bold()).width(Fill),
            action(Icon::Close, "Cancel", (!self.locked()).then_some(Msg::AddServer)).style(button::text)].spacing(12),
            self.field("Server address", "Hostname or IP, optionally with a port", &self.address, Field::Address)].spacing(16);
        if self.identity.is_some() {
            body = body.push(ui::badge("Server identity approved".into(), true))
                .push(self.field("Connection name", "A name for this server", &self.label, Field::Label))
                .push(column![ui::label_help("Account key", "The host supplies this private 20-character key with your account. Keep it secret. It replaces the game's original username/password login."),
                    text_input("Private key from the host", &self.key).secure(true).padding(12)
                        .on_input_maybe((!self.locked() && !self.modal_open()).then_some(|s| Msg::Input(Field::Key, s)))].spacing(8))
                .push(action(Icon::Check, "Save connection", (!self.locked() && self.key.trim().len() == 20).then_some(Msg::SaveServer)).style(button::primary));
        } else {
            body = body.push(muted("Check the server and confirm its identity before entering an account key."))
                .push(action(Icon::Shield, "Check server", (!self.locked() && !self.address.trim().is_empty()).then_some(Msg::Discover)).style(button::primary));
        }
        body.into()
    }
    fn server_view(&self) -> Element<'_, Msg> {
        let content: Element<'_, Msg> = match self.host_tab {
            HostTab::Overview => scrollable(self.host_overview()).height(Fill).into(),
            HostTab::Invitations => scrollable(self.accounts_view()).height(Fill).into(),
            HostTab::Advanced => column![scrollable(self.host_advanced()).height(Fill), self.save_actions(Section::Hosting)]
                .spacing(16).height(Fill).into(),
        };
        column![self.page_heading("My server", "Host independently while you play here or join another server."),
            row![ui::tab("Overview", Msg::HostTab(HostTab::Overview), self.host_tab == HostTab::Overview),
                ui::tab("Players & invitations", Msg::HostTab(HostTab::Invitations), self.host_tab == HostTab::Invitations),
                ui::tab("Advanced", Msg::HostTab(HostTab::Advanced), self.host_tab == HostTab::Advanced)].spacing(8),
            content].spacing(20).height(Fill).into()
    }
    fn host_overview(&self)->Element<'_,Msg>{
        let running=self.status.is_some();let status=self.status.as_ref();
        let mut body=column![card(column![row![text(if running{"Running on this computer"}else{"Ready to host"}).size(22).font(bold()).width(Fill),ui::badge(if running{"Running"}else{"Stopped"}.into(),running)].spacing(12),
            row![metric("Players online",status.and_then(|s|s["online"].as_u64()).unwrap_or(0)),metric("Active duels",status.and_then(|s|s["duels"].as_u64()).unwrap_or(0))].spacing(24),
            row![if running{action(Icon::Stop,"Stop server",(!self.busy && !self.applying).then_some(Msg::Stop)).style(button::danger)}else{action(Icon::Play,"Start server",(!self.locked()).then_some(Msg::Start)).style(button::primary)},action(Icon::Activity,"View activity",Some(Msg::Page(Page::Activity))).style(button::text)].spacing(12),
            muted("Starting uses saved configuration. Closing the game keeps hosting active.")].spacing(20))].spacing(20);
        let mut sharing=column![ui::label_help("Invite friends", "Create a separate account for each friend, then send them a private invitation. A sharing address must be reachable from their computer."),muted("Share a private invitation with each friend.")].spacing(14);
        match yo3_desktop::network::share_address(&self.settings){Ok(Some((address,_)))=>sharing=sharing.push(row![text(format!("Sharing address: {address}")).size(14).width(Fill),action(Icon::Copy,"Copy",Some(Msg::CopyAddress)).style(button::text)].spacing(12))
            .push(action(Icon::Users,"Players & invitations",Some(Msg::HostTab(HostTab::Invitations)))),_=>sharing=sharing.push(muted("A sharing address is not configured. localhost is for local play."))
            .push(action(Icon::Settings,"Configure sharing",Some(Msg::HostTab(HostTab::Advanced))))};
        sharing=sharing.push(action(Icon::Book,"Connection help",Some(Msg::NetworkHelp)).style(button::text));body=body.push(widget::rule::horizontal(1)).push(sharing);
        if let Some(status)=status {
            if let Some(problem)=status["engine_problem"].as_str(){body=body.push(text(problem).color(ui::BAD));}
            if let Some(players)=status["players"].as_array().filter(|p|!p.is_empty()){let mut people=column![text("Online now").size(18).font(bold())].spacing(12);
                for p in players{people=people.push(row![text(p["name"].as_str().unwrap_or("Player")).width(Fill),muted(if p["duelling"].as_bool().unwrap_or(false){"Duelling"}else{"Online"})].spacing(12));}body=body.push(people);}
        }
        body.into()
    }
    fn host_advanced(&self)->Element<'_,Msg>{
        let selected=self.bindings.iter().find(|b|b.interface_id==self.draft.bind_interface && b.address.to_string()==self.draft.bind_address).cloned();
        let mut body=column![ui::label_help("Sharing address", "The LAN/VPN address or public hostname friends use to join. Router port forwarding can make this different from the network interface and port below."),
            text_input("LAN/VPN address or public hostname", &self.draft.host_address).padding(12).on_input_maybe((!self.locked() && !self.modal_open()).then_some(|s|Msg::Input(Field::Host,s)))].spacing(16);
        let mut network=column![ui::label_help("Listen on", "The network interface the server accepts connections on. All interfaces (0.0.0.0) is the default. A specific interface limits access to that network; localhost allows only this computer."),muted("Stop the server before saving hosting changes."),
            pick_list(self.bindings.clone(),selected,Msg::BindSelect).placeholder("Choose interface").padding(12).width(Fill),
            row![column![muted("TCP port"),text_input("7443",&self.port).padding(12).on_input_maybe((!self.locked() && !self.modal_open()).then_some(|s|Msg::Input(Field::Port,s)))].spacing(8).width(150),action(Icon::Refresh,"Refresh interfaces",(!self.locked()).then_some(Msg::RefreshBindings))].spacing(12).align_y(iced::Alignment::End)].spacing(16);
        if let Some(listen)=self.status.as_ref().and_then(|s|s["listen"].as_str()){network=network.push(muted(format!("Active listener: {listen}")));}
        body=body.push(widget::rule::horizontal(1)).push(network).push(widget::rule::horizontal(1)).push(column![ui::label_help("Server data", "Accounts, cards and decks live in this folder. Leave it empty to use private application storage. Changing it selects a different server; it does not move existing data."),
            row![self.field("Data folder","Empty uses private application storage",&self.draft.data_dir,Field::Data),action(Icon::Folder,"Browse…",(!self.locked()).then_some(Msg::Browse(Field::Data)))].spacing(12).align_y(iced::Alignment::End),
            self.field("First player on a new server","Duelist",&self.draft.player_name,Field::Player),muted("Existing servers keep their accounts. Back up the complete data folder securely.")].spacing(16));
        body.into()
    }
    fn accounts_view(&self)->Element<'_,Msg>{
        if self.status.is_none(){return card(column![text("Start your server to manage players").size(20),muted("Accounts and invitations belong to your hosted server."),action(Icon::Server,"Go to overview",Some(Msg::HostTab(HostTab::Overview))).style(button::primary)].spacing(16)).into()}
        let names=account_names(&self.accounts);let share_ready=yo3_desktop::network::share_address(&self.settings).ok().flatten().is_some();
        let mut selected=column![row![ui::label_help("Account", "Manage this account’s players and invitation. This selection is separate from your account on Play."),widget::Space::new().width(Fill),action(Icon::Plus,"Create account",(!self.locked()).then_some(Msg::NewAccount)).style(button::text)].align_y(iced::Alignment::Center),pick_list(names,(!self.admin_account.is_empty()).then_some(self.admin_account.clone()),Msg::AdminAccountSelect).placeholder("Choose account").width(Fill).padding(12),
            muted("This selection manages invitations and players. Your account on Play is unchanged.")].spacing(16);
        if let Some(account)=self.accounts.as_array().and_then(|a|a.iter().find(|a|a["players"][0]["name"].as_str()==Some(&self.admin_account))){let players=account["players"].as_array().map(|p|p.iter().filter_map(|p|p["name"].as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();selected=selected.push(muted(format!("Players: {players} · {} BP",account["bp"])));}
        if share_ready{selected=selected.push(action(Icon::Upload,"Export private invitation…",(!self.locked() && !self.admin_account.is_empty()).then_some(Msg::Export)).style(button::primary));}
        else{selected=selected.push(muted("Configure a sharing address before exporting invitations."));}
        let mut body=column![selected].spacing(24);
        if self.account_form{body=body.push(widget::rule::horizontal(1)).push(column![self.field("First player's name","1–16 characters",&self.new_account,Field::Account),action(Icon::Check,"Create account",(!self.locked() && !self.new_account.trim().is_empty()).then_some(Msg::CreateAccount)).style(button::primary)].spacing(16));}
        body=body.push(disclosure("Additional players",self.account_options,Msg::Disclosure(Disclosure::Account)));
        if self.account_options{body=body.push(column![muted("Each account can hold up to four players. Choose the player inside the game."),self.field("Player name","New player on the selected account",&self.new_player,Field::NewPlayer),action(Icon::Plus,"Add player",(!self.locked() && !self.admin_account.is_empty() && !self.new_player.trim().is_empty()).then_some(Msg::AddPlayer))].spacing(16));}
        body.into()
    }
    fn activity_view(&self)->Element<'_,Msg>{
        let shown = self.shown_logs();
        let mut body=column![self.page_heading("Activity","View session logs or export a diagnostic report."),row![ui::tab("Server",Msg::Logs(true),self.log_server),ui::tab("Game & launcher",Msg::Logs(false),!self.log_server),widget::Space::new().width(Fill),action(Icon::Download,"Export logs…",(!self.locked()).then_some(Msg::ExportLogs))].spacing(8),
            row![text_input("Filter activity…",&self.filter).on_input(Msg::LogFilter).padding(12).width(Fill),checkbox(self.paused).label("Pause").on_toggle(Msg::Pause),action(Icon::Copy,"Copy",Some(Msg::CopyLogs)).style(button::text)].spacing(12).align_y(iced::Alignment::Center),
            card(scrollable(text(if shown.is_empty(){"No activity to show yet.".into()}else{shown}).font(iced::Font::MONOSPACE).size(12)).id("activity-log").height(Fill)).height(Fill),
            disclosure("Diagnostics options",self.advanced_diagnostics,Msg::Disclosure(Disclosure::Diagnostics))].spacing(16).height(Fill);
        if self.advanced_diagnostics{body=body.push(column![checkbox(self.draft.detailed_logs).label("Detailed client and protocol tracing").on_toggle(Msg::Detailed),muted("Applies to the next game/server start. Detailed traces can contain chat/player information; turn this off after investigating."),self.save_actions(Section::Diagnostics),action(Icon::Folder,"Open application data",Some(Msg::OpenData)).style(button::text)].spacing(12));}
        body.into()
    }
    fn settings_view(&self) -> Element<'_, Msg> {
        let changed = self.changed(Section::Game);
        let mut game = column![row![ui::label_help("Game installation", "Choose the complete original Yu-Gi-Oh! ONLINE 3 installation, including yo3.exe, yo3.idx and package/. The launcher checks the saved game and its connection components before Play."),
                widget::Space::new().width(Fill), self.setup_badge()].spacing(12).align_y(iced::Alignment::Center),
            row![self.field("Game folder", "Folder containing yo3.exe", &self.draft.game_dir, Field::Game),
                action(Icon::Folder, "Browse…", (!self.locked()).then_some(Msg::Browse(Field::Game)))].spacing(12).align_y(iced::Alignment::End),
            disclosure("Advanced game options", self.advanced_game, Msg::Disclosure(Disclosure::Game))].spacing(16);
        if self.advanced_game && !cfg!(windows) {
            game = game.push(column![ui::label_help("Wine prefix", "Linux only: a separate Wine environment containing the game's registry and Windows files. Leave empty to use your default Wine prefix."),
                text_input("Empty uses the default prefix", &self.draft.wine_prefix).padding(12)
                    .on_input_maybe((!self.locked() && !self.modal_open()).then_some(|s| Msg::Input(Field::Wine, s)))].spacing(8));
        }
        game = game.push(self.save_actions(Section::Game));
        let detail = match &self.setup_result {
            None => "Checking game files and required connection components…",
            Some(yo3_desktop::Readiness::Ready) => "The saved game files and matching connection components passed preflight.",
            Some(yo3_desktop::Readiness::GameProblem(e) | yo3_desktop::Readiness::ComponentProblem(e)) => e,
        };
        let mut checks = row![action(Icon::Refresh, "Check setup", (!self.locked()).then_some(Msg::CheckSetup)).style(button::text)].spacing(12);
        if self.setup_result.as_ref().is_some_and(|s| *s != yo3_desktop::Readiness::Ready) {
            checks = checks.push(action(Icon::Settings, "Repair…", (!self.locked()).then_some(Msg::RepairSetup)).style(button::text));
        }
        let mut body = column![self.page_heading("Settings", "Set up your game and optional direct launch."), game, muted(detail), checks].spacing(20);
        if self.first_setup && !changed && matches!(self.setup_result, Some(yo3_desktop::Readiness::Ready)) {
            body = body.push(action(Icon::Play, "Continue to Play", Some(Msg::Page(Page::Play))).style(button::primary));
        }
        if self.advanced_game {
            let (label, good) = match self.direct_state {
                yo3_desktop::direct::State::Disabled => ("Disabled", false),
                yo3_desktop::direct::State::Enabled => ("Enabled", true),
                yo3_desktop::direct::State::NeedsRepair(_) => ("Needs repair", false),
                yo3_desktop::direct::State::Conflict(_) => ("Mod conflict", false),
            };
            let mut direct = column![row![ui::label_help("Direct launch", "Optional setup for opening yo3.exe or its original shortcut without this window. A small reversible loader checks the saved server and connection DLL before login. The original executable and INI are unchanged."),
                    widget::Space::new().width(Fill), ui::badge(label.into(), good)].spacing(12),
                muted("Optional: open yo3.exe or its original shortcut with your saved server and account.")].spacing(16);
            let idle = !self.locked() && !self.game_running && !changed;
            let ready = idle && matches!(self.setup_result, Some(yo3_desktop::Readiness::Ready));
            match &self.direct_state {
                yo3_desktop::direct::State::Disabled => direct = direct.push(action(Icon::Play, "Enable direct launch", ready.then_some(Msg::Direct(true))).style(button::primary)),
                yo3_desktop::direct::State::Enabled => direct = direct.push(action(Icon::Close, "Disable direct launch", idle.then_some(Msg::Direct(false))).style(button::danger)),
                yo3_desktop::direct::State::NeedsRepair(reason) => direct = direct.push(muted(reason)).push(row![
                    action(Icon::Refresh, "Repair direct launch", ready.then_some(Msg::Direct(true))).style(button::primary),
                    action(Icon::Close, "Disable", idle.then_some(Msg::Direct(false))).style(button::danger)].spacing(12)),
                yo3_desktop::direct::State::Conflict(reason) => direct = direct.push(muted(reason)),
            }
            if changed { direct = direct.push(muted("Save or cancel the game changes before modifying direct launch.")); }
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
        let mut body = column![self.page_heading("About & updates", "Stay current with YO3 Revival."),
            column![row![text(format!("YO3 Revival {}", yo3_tunnel::BUILD_VERSION)).size(23).font(bold()).width(Fill),
                action(Icon::Copy, "Copy version", Some(Msg::CopyVersion)).style(button::text)].spacing(12),
                muted(format!("Build {} · {}", yo3_tunnel::BUILD_COMMIT, std::env::consts::OS))].spacing(12),
            widget::rule::horizontal(1), column![text(title).size(20).font(bold()), muted(detail),
                row![action(Icon::Refresh, "Check for updates", (!matches!(self.update_status, UpdateStatus::Checking)).then_some(Msg::CheckUpdates)),
                    action(Icon::External, "Releases", Some(Msg::ReleasePage)).style(button::text)].spacing(12)].spacing(16)].spacing(28);
        if let UpdateStatus::Checked(updates::Decision::Available(r)) = &self.update_status {
            body = body.push(widget::rule::horizontal(1)).push(column![text("Release notes").size(18).font(bold()), text(&r.notes).size(14),
                muted(format!("Download size: {}", size(r.asset.bytes))),
                action(Icon::Download, "Review & install…", (!self.locked()).then_some(Msg::ReviewUpdate)).style(button::primary)].spacing(16));
        }
        body=body.push(muted("Startup checks leave Play available. Download and installation always require your confirmation."));page_scroll(body)
    }
    fn setup_badge(&self)->Element<'_,Msg>{let (label,good)=match &self.setup_result{None=>("Checking setup",false),Some(yo3_desktop::Readiness::Ready)=>("Ready to play",true),Some(yo3_desktop::Readiness::GameProblem(_))=>("Game setup needed",false),Some(yo3_desktop::Readiness::ComponentProblem(_))=>("Connection components need repair",false)};ui::badge(label.into(),good)}
    fn save_actions(&self,section:Section)->Element<'_,Msg>{let changed=self.changed(section);row![action(Icon::Check,"Save changes",(!self.locked() && changed).then_some(Msg::SaveSettings(section))).style(button::primary),action(Icon::Close,"Cancel",(!self.locked() && changed).then_some(Msg::CancelSettings(section))).style(button::text),muted(if changed{"Unsaved changes"}else{"Saved"})].spacing(12).align_y(iced::Alignment::Center).into()}
    fn field<'a>(&self,label:&'static str,hint:&'static str,value:&str,field:Field)->Element<'a,Msg>{column![muted(label),text_input(hint,value).padding(12).on_input_maybe((!self.locked() && !self.modal_open()).then_some(move |v|Msg::Input(field,v)))].spacing(8).width(Fill).into()}
}
fn bold()->iced::Font{iced::Font{weight:iced::font::Weight::Semibold,..iced::Font::with_name("Fira Sans")}}
fn heading<'a>(title:&'static str,description:&'static str)->Element<'a,Msg>{column![text(title).size(28).font(bold()),muted(description)].spacing(8).into()}
fn metric<'a>(label:&'static str,value:u64)->Element<'a,Msg>{column![text(value.to_string()).size(26),muted(label)].spacing(4).into()}
fn page_scroll(body:widget::Column<'_,Msg>)->Element<'_,Msg>{scrollable(body.width(Fill)).spacing(16).width(Fill).height(Fill).into()}
fn size(bytes:u64)->String{if bytes<1_000_000{format!("{:.1} KB",bytes as f64/1000.0)}else{format!("{:.1} MB",bytes as f64/1_000_000.0)}}
