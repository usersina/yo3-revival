use yo3_desktop::{Settings, server_options};
#[test]
fn advanced_flags_are_typed_validated_and_never_parsed_as_a_shell_command() {
    let mut settings=Settings::default();
    settings.server_options.insert("duel-check".into(),"true".into());
    settings.server_options.insert("duel-time".into(),"0,0".into());
    settings.server_options.insert("auth-cert".into(),"my cert.crt; echo literal".into());
    let args=server_options::arguments(&settings).unwrap();
    assert!(args.contains(&"--duel-check".into())); assert!(args.contains(&"--duel-time=0,0".into()));
    assert!(args.contains(&"--auth-cert=my cert.crt; echo literal".into()));
    settings.server_options.insert("duel-lp".into(),"invalid".into()); assert!(server_options::arguments(&settings).is_err());
    settings.server_options.remove("duel-lp");settings.server_options.insert("made-up".into(),"true".into());assert!(server_options::arguments(&settings).is_err());
}
#[test]
fn app_owned_options_have_one_owner_and_duel_validation_is_an_explicit_option() {
    let mut settings=Settings::default();
    settings.server_options.insert("duel-check".into(),"false".into());
    assert!(!server_options::arguments(&settings).unwrap().contains(&"--duel-check".into()));
    settings.server_options.insert("data-dir".into(),"elsewhere".into()); assert!(server_options::arguments(&settings).is_err());
    assert!(server_options::managed("data-dir").is_some()); assert!(server_options::managed("duel-check").is_none());
    // The banlist has its own control (Hosting → Overview → Banlist), so the flag is refused here.
    settings.server_options.remove("data-dir"); settings.server_options.insert("regulation".into(),"3".into());
    assert!(server_options::arguments(&settings).is_err() && server_options::managed("regulation").is_some());
}
