//! Message entries, option lists, declarations and the hints' words.
use super::*;

/// A data command per four values, which the pop appends in order (0x522A50).
pub(super) fn fours(id: u32, values: &[u32]) -> Vec<Message> {
    let command_of = |four: &[u32]| {
        let mut params = [0; 4];
        params[..four.len()].copy_from_slice(four);
        command(id, params)
    };
    values.chunks(4).map(command_of).collect()
}

/// The option list for MSG_SELECT_OPTION {options}, when the client has a label for every option (at most four).
/// A card's own strings (code << 20 | n) have labels only where CARD_OPTION_LABELS pairs them.
pub(super) fn option_list(o: &SelectOption) -> Option<Vec<Message>> {
    let options = &o.options;
    let label = |d: u64| {
        let system = OPTION_LABELS.iter().find(|(o, _, _)| *o == d).map(|&(_, label, title)| (label, title));
        system.or_else(|| CARD_OPTION_LABELS.iter().find(|(o, _)| *o == d).map(|&(_, label)| (label, text::SELECT_EFFECT)))
    };
    let found: Option<Vec<(u32, u32)>> = options.iter().map(|&o| label(o)).collect();
    let found = found.filter(|f| !f.is_empty() && f.len() <= SELECT_MAX)?;
    let labels: Vec<u32> = found.iter().map(|(label, _)| label | dialog::LISTED).collect();
    let mut out = vec![command(cmd::LIST_BEGIN, [labels.len() as u32, 0, 0, 0])];
    out.extend(fours(cmd::LIST_VALUES, &labels));
    out.extend(message(&[entry::TEXT | found[0].1]));
    out.push(Cmd::Dialog { kind: dialog::SELECT, message: dialog::FROM_ENTRIES, param: 0 }.message());
    Some(out)
}

/// The message entries (at most four) a dialog shows: 0x3EC {count}, 0x3ED {entries}.
pub(super) fn message(entries: &[u32]) -> Vec<Message> {
    let mut params = [0; 4];
    params[..entries.len()].copy_from_slice(entries);
    vec![command(cmd::MESSAGE_LENGTH, [entries.len() as u32, 0, 0, 0]), command(cmd::MESSAGE, params)]
}

/// The client's dialog for a declaration (MSG_ANNOUNCE_RACE / ATTRIB {count, available}, NUMBER {options}, CARD):
/// None when it can't ask it in one window (several types at once, numbers outside the number dialog's range:
/// `declaration_steps` asks those in steps). `level`: the number is
/// a Level (hint 567 before it), which the dialog says; Duel.AnnounceLevel is only an alias of AnnounceNumberRange, and
/// the client's other cards declare counts and die rolls with it.
pub(super) fn declaration(ask: &Ask, level: bool) -> Option<Vec<Message>> {
    let bits = |allowed: u64, table: &[u64]| table.iter().enumerate().filter(|(_, b)| allowed & **b != 0).fold(0u32, |f, (i, _)| f | 1 << (i + 1));
    let (dialog, text, param3) = match ask {
        // A dialog with no type to pick (the client has none of the available ones) would hold the player.
        Ask::Race(a) if a.count == 1 && bits(a.available, &TYPES) != 0 => (dialog::TYPE, text::TYPE, bits(a.available, &TYPES)),
        Ask::Attrib(a) if a.count > 0 && bits(a.available, &ATTRIBUTES).count_ones() as u64 >= a.count => {
            (dialog::ATTRIBUTE, text::ATTRIBUTE, bits(a.available, &ATTRIBUTES) << 16 | a.count as u32)
        }
        Ask::Number(n) => {
            let options = &n.options;
            let (unit, format) = number_unit(options)?;
            let (min, max) = (options.iter().min()? / unit, options.iter().max()? / unit);
            let step = options.windows(2).map(|w| w[1].abs_diff(w[0]) / unit).filter(|&d| d > 0).min().unwrap_or(1);
            if min > 15 || step > 15 || max > 255 {
                return None;
            }
            let param3 = (max as u32) << 24 | (step as u32) << 20 | (min as u32) << 16 | format;
            // A Level gets "Declare monster level."; other numbers no message.
            if level {
                let mut out = message(&[entry::TEXT | text::LEVEL]);
                out.push(Cmd::Dialog { kind: dialog::NUMBER, message: dialog::FROM_ENTRIES, param: param3 }.message());
                return Some(out);
            }
            return Some(vec![Cmd::Dialog { kind: dialog::NUMBER, message: 0, param: param3 }.message()]);
        }
        Ask::CardName(_) => return Some(vec![command(cmd::LIST_BEGIN, [0, 1, 0, 0]), command(cmd::CARD_LIST, [CLIENT, list::SEARCH, 0, 0])]),
        _ => return None,
    };
    let mut out = message(&[entry::TEXT | text]);
    out.push(Cmd::Dialog { kind: dialog, message: dialog::FROM_ENTRIES, param: param3 }.message());
    Some(out)
}

/// The client's number for EDOPro's Type or Attribute bits: its place in the client's list, plus one (CARD_Prop and the
/// declaration dialogs number them so). None without bits or for one the client lacks.
pub(super) fn client_number(bits: Option<u64>, table: &[u64]) -> Option<u32> {
    let bits = bits.filter(|&b| b != 0)?;
    table.iter().position(|&t| bits & t != 0).map(|i| i as u32 + 1)
}

/// The number dialog's unit and format for these options: the largest unit all of them are multiples of.
pub(super) fn number_unit(options: &[u64]) -> Option<(u64, u32)> {
    if options.is_empty() {
        return None;
    }
    NUMBER_FORMATS.iter().copied().find(|(unit, _)| options.iter().all(|o| o % unit == 0))
}

/// The duel prompt table's text for an EDOPro selection hint (HINTMSG_*), where one says the same thing.
pub(super) fn hint_text(hint: u64) -> Option<u32> {
    Some(match hint {
        hintmsg::RELEASE => text::TRIBUTE,
        501 => 347, // HINTMSG_DISCARD: "Select the @2card(s)@0 in your hand you want to discard."
        502 => 281, // HINTMSG_DESTROY: "Select @3card@0 to @3destroy@0."
        503 => 70,  // HINTMSG_REMOVE: "Select @3card(s)@0 to @3remove from play@0."
        504 => 58,  // HINTMSG_TOGRAVE: "Select the @3card@0 to send to the @2Graveyard@0."
        505 => 348, // HINTMSG_RTOHAND: "Select the @3card(s)@0 to return to the @2hand@0."
        506 => 88,  // HINTMSG_ATOHAND: "Select the @3card(s)@0 to add to your @2hand@0."
        507 => 78,  // HINTMSG_TODECK: "Select a @3card@0 to be returned the owner's @2Deck@0."
        508 => 39,  // HINTMSG_SUMMON: "Select a @3monster@0 to @3Normal Summon or Set@0."
        509 => 25,  // HINTMSG_SPSUMMON: "Select the @3monster(s)@0 to @3Special Summon@0."
        510 => 46,  // HINTMSG_SET: "Select a @3Spell or Trap Card@0 to @3Set@0 on the field."
        511 => 92,  // HINTMSG_FMATERIAL: "Select @3monsters@0 to use as @3Fusion Material@0."
        // HINTMSG_FACEUP: the client has no "face-up card" prompt; EDOPro asks it for targets (Spellbinding Circle).
        514 => 94,
        515 => 273, // HINTMSG_FACEDOWN: "Select the @3monster(s)@0 in @3face-down@0 Position."
        518 => 101, // HINTMSG_EQUIP: "Select a @3monster@0 to @3equip@0."
        520 => 119, // HINTMSG_CONTROL: "Select the @3monster(s)@0 your opponent controls to take control."
        526 => 341, // HINTMSG_CONFIRM: "Select the @3card(s)@0 to show your opponent."
        528 => 357, // HINTMSG_POSCHANGE: "Select the @3monster@0 to change its battle position."
        549 => 222, // HINTMSG_ATTACKTARGET: "Select an attack target @3monster@0."
        550 => text::SELECT_EFFECT, // HINTMSG_EFFECT: "Select effect to @3activate@0."
        551 => 94,  // HINTMSG_TARGET: "Select the @3card(s)@0 to target."
        _ => return None,
    })
}

/// The server's words for EDOPro's selection hints (HINTMSG_*) that the client's prompt table has no text for, in
/// the table's style (`@3` highlights, `@2` places), in the game's languages: English, Japanese, German, French,
/// Italian, Spanish, Korean (#92: a Japanese game showed the English lines). Shown through the connection DLL
/// (`Translator::text`).
pub(super) const HINT_WORDS: [(u64, [&str; 7]); 38] = [
    (512, [
        "Select the @3monster(s)@0 to use as @3Synchro Material@0.",
        "@3シンクロ素材@0にする@3モンスター@0を選択してください。",
        "Wähle die @3Monster@0, die als @3Synchro-Material@0 verwendet werden.",
        "Sélectionnez le(s) @3monstre(s)@0 à utiliser comme @3Matériel de Synchro@0.",
        "Seleziona i @3mostri@0 da usare come @3Materiale Synchro@0.",
        "Selecciona los @3monstruos@0 que usarás como @3Material de Sincronía@0.",
        "@3싱크로 소재@0로 할 @3몬스터@0를 선택해 주세요",
    ]),
    (513, [
        "Select the @3monsters@0 to use as @3Xyz Material@0.",
        "@3エクシーズ素材@0にする@3モンスター@0を選択してください。",
        "Wähle die @3Monster@0, die als @3Xyz-Material@0 verwendet werden.",
        "Sélectionnez les @3monstres@0 à utiliser comme @3Matériel Xyz@0.",
        "Seleziona i @3mostri@0 da usare come @3Materiale Xyz@0.",
        "Selecciona los @3monstruos@0 que usarás como @3Material Xyz@0.",
        "@3엑시즈 소재@0로 할 @3몬스터@0를 선택해 주세요",
    ]),
    (516, [
        "Select a @3monster@0 in @3Attack Position@0.",
        "@3攻撃表示@0の@3モンスター@0を選択してください。",
        "Wähle ein @3Monster@0 in @3Angriffsposition@0.",
        "Sélectionnez un @3monstre@0 en @3Position d'Attaque@0.",
        "Seleziona un @3mostro@0 in @3Posizione di Attacco@0.",
        "Selecciona un @3monstruo@0 en @3Posición de Ataque@0.",
        "@3공격 표시@0인 @3몬스터@0를 선택해 주세요",
    ]),
    (517, [
        "Select a @3monster@0 in @3Defense Position@0.",
        "@3守備表示@0の@3モンスター@0を選択してください。",
        "Wähle ein @3Monster@0 in @3Verteidigungsposition@0.",
        "Sélectionnez un @3monstre@0 en @3Position de Défense@0.",
        "Seleziona un @3mostro@0 in @3Posizione di Difesa@0.",
        "Selecciona un @3monstruo@0 en @3Posición de Defensa@0.",
        "@3수비 표시@0인 @3몬스터@0를 선택해 주세요",
    ]),
    (519, [
        "Select the @3Xyz Material@0 to detach.",
        "取り除く@3エクシーズ素材@0を選択してください。",
        "Wähle das @3Xyz-Material@0, das abgehängt wird.",
        "Sélectionnez le @3Matériel Xyz@0 à détacher.",
        "Seleziona il @3Materiale Xyz@0 da staccare.",
        "Selecciona el @3Material Xyz@0 que vas a desacoplar.",
        "제거할 @3엑시즈 소재@0를 선택해 주세요",
    ]),
    (521, [
        "Select the @3card@0 to destroy instead.",
        "代わりに@3破壊@0する@3カード@0を選択してください。",
        "Wähle die @3Karte@0, die stattdessen @3zerstört@0 wird.",
        "Sélectionnez la @3carte@0 à @3détruire@0 à la place.",
        "Seleziona la @3carta@0 da @3distruggere@0 al suo posto.",
        "Selecciona la @3carta@0 que vas a @3destruir@0 en su lugar.",
        "대신 @3파괴@0할 @3카드@0를 선택해 주세요",
    ]),
    (522, [
        "Select a @3monster@0 in face-up @3Attack Position@0.",
        "表側@3攻撃表示@0の@3モンスター@0を選択してください。",
        "Wähle ein offenes @3Monster@0 in @3Angriffsposition@0.",
        "Sélectionnez un @3monstre@0 face recto en @3Position d'Attaque@0.",
        "Seleziona un @3mostro@0 scoperto in @3Posizione di Attacco@0.",
        "Selecciona un @3monstruo@0 boca arriba en @3Posición de Ataque@0.",
        "앞면 @3공격 표시@0인 @3몬스터@0를 선택해 주세요",
    ]),
    (523, [
        "Select a @3monster@0 in face-up @3Defense Position@0.",
        "表側@3守備表示@0の@3モンスター@0を選択してください。",
        "Wähle ein offenes @3Monster@0 in @3Verteidigungsposition@0.",
        "Sélectionnez un @3monstre@0 face recto en @3Position de Défense@0.",
        "Seleziona un @3mostro@0 scoperto in @3Posizione di Difesa@0.",
        "Selecciona un @3monstruo@0 boca arriba en @3Posición de Defensa@0.",
        "앞면 @3수비 표시@0인 @3몬스터@0를 선택해 주세요",
    ]),
    (524, [
        "Select a @3monster@0 in face-down @3Attack Position@0.",
        "裏側@3攻撃表示@0の@3モンスター@0を選択してください。",
        "Wähle ein verdecktes @3Monster@0 in @3Angriffsposition@0.",
        "Sélectionnez un @3monstre@0 face verso en @3Position d'Attaque@0.",
        "Seleziona un @3mostro@0 coperto in @3Posizione di Attacco@0.",
        "Selecciona un @3monstruo@0 boca abajo en @3Posición de Ataque@0.",
        "뒷면 @3공격 표시@0인 @3몬스터@0를 선택해 주세요",
    ]),
    (525, [
        "Select a @3monster@0 in face-down @3Defense Position@0.",
        "裏側@3守備表示@0の@3モンスター@0を選択してください。",
        "Wähle ein verdecktes @3Monster@0 in @3Verteidigungsposition@0.",
        "Sélectionnez un @3monstre@0 face verso en @3Position de Défense@0.",
        "Seleziona un @3mostro@0 coperto in @3Posizione di Difesa@0.",
        "Selecciona un @3monstruo@0 boca abajo en @3Posición de Defensa@0.",
        "뒷면 @3수비 표시@0인 @3몬스터@0를 선택해 주세요",
    ]),
    (527, [
        "Select the @3card@0 to place on the field.",
        "@2フィールド@0に置く@3カード@0を選択してください。",
        "Wähle die @3Karte@0, die aufs @2Spielfeld@0 gelegt wird.",
        "Sélectionnez la @3carte@0 à placer sur le @2Terrain@0.",
        "Seleziona la @3carta@0 da mettere sul @2Terreno@0.",
        "Selecciona la @3carta@0 que vas a colocar en el @2Campo@0.",
        "@2필드@0에 놓을 @3카드@0를 선택해 주세요",
    ]),
    (529, [
        "Select a @3card@0 you control.",
        "自分の@3カード@0を選択してください。",
        "Wähle eine @3Karte@0, die du kontrollierst.",
        "Sélectionnez une @3carte@0 que vous contrôlez.",
        "Seleziona una @3carta@0 che controlli.",
        "Selecciona una @3carta@0 que controles.",
        "자신의 @3카드@0를 선택해 주세요",
    ]),
    (530, [
        "Select a @3card@0 your opponent controls.",
        "相手の@3カード@0を選択してください。",
        "Wähle eine @3Karte@0, die dein Gegner kontrolliert.",
        "Sélectionnez une @3carte@0 contrôlée par votre adversaire.",
        "Seleziona una @3carta@0 controllata dal tuo avversario.",
        "Selecciona una @3carta@0 que controle tu adversario.",
        "상대의 @3카드@0를 선택해 주세요",
    ]),
    (531, [
        "Select the @3monster(s)@0 to @3Tribute@0.",
        "@3リリース@0する@3モンスター@0を選択してください。",
        "Wähle die @3Monster@0, die als @3Tribut@0 angeboten werden.",
        "Sélectionnez le(s) @3monstre(s)@0 à @3Sacrifier@0.",
        "Seleziona i @3mostri@0 da @3offrire come Tributo@0.",
        "Selecciona los @3monstruos@0 que vas a @3Sacrificar@0.",
        "@3릴리스@0할 @3몬스터@0를 선택해 주세요",
    ]),
    (532, [
        "Select the @3Xyz Monster@0 to detach material from.",
        "素材を取り除く@3エクシーズモンスター@0を選択してください。",
        "Wähle das @3Xyz-Monster@0, von dem Material abgehängt wird.",
        "Sélectionnez le @3Monstre Xyz@0 dont détacher un matériel.",
        "Seleziona il @3Mostro Xyz@0 da cui staccare un materiale.",
        "Selecciona el @3Monstruo Xyz@0 del que vas a desacoplar un material.",
        "소재를 제거할 @3엑시즈 몬스터@0를 선택해 주세요",
    ]),
    (533, [
        "Select the @3monsters@0 to use as @3Link Material@0.",
        "@3リンク素材@0にする@3モンスター@0を選択してください。",
        "Wähle die @3Monster@0, die als @3Link-Material@0 verwendet werden.",
        "Sélectionnez les @3monstres@0 à utiliser comme @3Matériel de Lien@0.",
        "Seleziona i @3mostri@0 da usare come @3Materiale Link@0.",
        "Selecciona los @3monstruos@0 que usarás como @3Material de Enlace@0.",
        "@3링크 소재@0로 할 @3몬스터@0를 선택해 주세요",
    ]),
    (552, [
        "Call heads or tails.",
        "コインの表か裏を宣言してください。",
        "Sage Kopf oder Zahl an.",
        "Annoncez pile ou face.",
        "Scegli testa o croce.",
        "Elige cara o cruz.",
        "코인의 앞면 또는 뒷면을 선언해 주세요",
    ]),
    (553, [
        "Select a die result.",
        "サイコロの目を選択してください。",
        "Wähle ein Würfelergebnis.",
        "Sélectionnez un résultat de dé.",
        "Seleziona un risultato del dado.",
        "Selecciona un resultado del dado.",
        "주사위 눈을 선택해 주세요",
    ]),
    (554, [
        "Declare a @3card type@0.",
        "@3カードの種類@0を宣言してください。",
        "Sage einen @3Kartentyp@0 an.",
        "Déclarez un @3type de carte@0.",
        "Dichiara un @3tipo di carta@0.",
        "Declara un @3tipo de carta@0.",
        "@3카드의 종류@0를 선언해 주세요",
    ]),
    (555, [
        "Select an option.",
        "選択肢を選んでください。",
        "Wähle eine Option.",
        "Sélectionnez une option.",
        "Seleziona un'opzione.",
        "Selecciona una opción.",
        "선택지를 골라 주세요",
    ]),
    (556, [
        "Select the effect to apply.",
        "適用する効果を選択してください。",
        "Wähle den Effekt, der angewendet wird.",
        "Sélectionnez l'effet à appliquer.",
        "Seleziona l'effetto da applicare.",
        "Selecciona el efecto que se aplicará.",
        "적용할 효과를 선택해 주세요",
    ]),
    (560, [
        "Select a @3card@0.",
        "@3カード@0を選択してください。",
        "Wähle eine @3Karte@0.",
        "Sélectionnez une @3carte@0.",
        "Seleziona una @3carta@0.",
        "Selecciona una @3carta@0.",
        "@3카드@0를 선택해 주세요",
    ]),
    (561, [
        "Select a battle position.",
        "表示形式を選択してください。",
        "Wähle eine Kampfposition.",
        "Sélectionnez une position de combat.",
        "Seleziona una posizione di battaglia.",
        "Selecciona una posición de batalla.",
        "표시 형식을 선택해 주세요",
    ]),
    (562, [
        "Declare an @3Attribute@0.",
        "@3属性@0を宣言してください。",
        "Sage eine @3Eigenschaft@0 an.",
        "Déclarez un @3Attribut@0.",
        "Dichiara un @3Attributo@0.",
        "Declara un @3Atributo@0.",
        "@3속성@0을 선언해 주세요",
    ]),
    (563, [
        "Declare a @3Type@0.",
        "@3種族@0を宣言してください。",
        "Sage einen @3Typ@0 an.",
        "Déclarez un @3Type@0.",
        "Dichiara un @3Tipo@0.",
        "Declara un @3Tipo@0.",
        "@3종족@0을 선언해 주세요",
    ]),
    (564, [
        "Declare a @3card name@0.",
        "@3カード名@0を宣言してください。",
        "Sage einen @3Kartennamen@0 an.",
        "Déclarez un @3nom de carte@0.",
        "Dichiara il @3nome di una carta@0.",
        "Declara el @3nombre de una carta@0.",
        "@3카드명@0을 선언해 주세요",
    ]),
    (565, [
        "Declare a number.",
        "数字を宣言してください。",
        "Sage eine Zahl an.",
        "Déclarez un nombre.",
        "Dichiara un numero.",
        "Declara un número.",
        "숫자를 선언해 주세요",
    ]),
    (566, [
        "Select the @3effect@0 to activate.",
        "発動する@3効果@0を選択してください。",
        "Wähle den @3Effekt@0, der aktiviert wird.",
        "Sélectionnez l'@3effet@0 à activer.",
        "Seleziona l'@3effetto@0 da attivare.",
        "Selecciona el @3efecto@0 que vas a activar.",
        "발동할 @3효과@0를 선택해 주세요",
    ]),
    (567, [
        "Declare a @3Level@0.",
        "@3レベル@0を宣言してください。",
        "Sage eine @3Stufe@0 an.",
        "Déclarez un @3Niveau@0.",
        "Dichiara un @3Livello@0.",
        "Declara un @3Nivel@0.",
        "@3레벨@0을 선언해 주세요",
    ]),
    (568, [
        "Select the @3card@0 whose effect to apply.",
        "効果を適用する@3カード@0を選択してください。",
        "Wähle die @3Karte@0, deren Effekt angewendet wird.",
        "Sélectionnez la @3carte@0 dont l'effet s'applique.",
        "Seleziona la @3carta@0 di cui applicare l'effetto.",
        "Selecciona la @3carta@0 cuyo efecto se aplicará.",
        "효과를 적용할 @3카드@0를 선택해 주세요",
    ]),
    (569, [
        "Select a @3zone@0.",
        "@3ゾーン@0を選択してください。",
        "Wähle eine @3Zone@0.",
        "Sélectionnez une @3zone@0.",
        "Seleziona una @3zona@0.",
        "Selecciona una @3zona@0.",
        "@3존@0을 선택해 주세요",
    ]),
    (570, [
        "Select a @3zone@0 to make unusable.",
        "使用できなくする@3ゾーン@0を選択してください。",
        "Wähle eine @3Zone@0, die unbenutzbar wird.",
        "Sélectionnez une @3zone@0 à rendre inutilisable.",
        "Seleziona una @3zona@0 da rendere inutilizzabile.",
        "Selecciona una @3zona@0 que quedará inutilizable.",
        "사용할 수 없게 할 @3존@0을 선택해 주세요",
    ]),
    (572, [
        "Select the @3card@0 to place a @3counter@0 on.",
        "@3カウンター@0を置く@3カード@0を選択してください。",
        "Wähle die @3Karte@0, auf die eine @3Marke@0 gelegt wird.",
        "Sélectionnez la @3carte@0 sur laquelle placer un @3compteur@0.",
        "Seleziona la @3carta@0 su cui mettere un @3segnalino@0.",
        "Selecciona la @3carta@0 en la que colocarás un @3contador@0.",
        "@3카운터@0를 놓을 @3카드@0를 선택해 주세요",
    ]),
    (575, [
        "Select the @3card@0 to negate.",
        "@3無効@0にする@3カード@0を選択してください。",
        "Wähle die @3Karte@0, die @3annulliert@0 wird.",
        "Sélectionnez la @3carte@0 à @3annuler@0.",
        "Seleziona la @3carta@0 da @3annullare@0.",
        "Selecciona la @3carta@0 que vas a @3negar@0.",
        "@3무효@0로 할 @3카드@0를 선택해 주세요",
    ]),
    (576, [
        "Select the @3monster@0 whose @3ATK/DEF@0 changes.",
        "@3攻撃力／守備力@0を変更する@3モンスター@0を選択してください。",
        "Wähle das @3Monster@0, dessen @3ATK/DEF@0 sich ändert.",
        "Sélectionnez le @3monstre@0 dont l'@3ATK/DEF@0 change.",
        "Seleziona il @3mostro@0 di cui cambiano @3ATK/DEF@0.",
        "Selecciona el @3monstruo@0 cuyo @3ATK/DEF@0 cambiará.",
        "@3공격력/수비력@0을 변경할 @3몬스터@0를 선택해 주세요",
    ]),
    (577, [
        "Select the @3card@0 to apply the effect to.",
        "効果を適用する@3カード@0を選択してください。",
        "Wähle die @3Karte@0, auf die der Effekt angewendet wird.",
        "Sélectionnez la @3carte@0 à laquelle appliquer l'effet.",
        "Seleziona la @3carta@0 a cui applicare l'effetto.",
        "Selecciona la @3carta@0 a la que se aplicará el efecto.",
        "효과를 적용할 @3카드@0를 선택해 주세요",
    ]),
    (578, [
        "Select the @3card(s)@0 to attach as @3Xyz Material@0.",
        "@3エクシーズ素材@0にする@3カード@0を選択してください。",
        "Wähle die @3Karte(n)@0, die als @3Xyz-Material@0 angehängt werden.",
        "Sélectionnez la/les @3carte(s)@0 à attacher comme @3Matériel Xyz@0.",
        "Seleziona le @3carte@0 da collegare come @3Materiale Xyz@0.",
        "Selecciona las @3cartas@0 que vas a acoplar como @3Material Xyz@0.",
        "@3엑시즈 소재@0로 할 @3카드@0를 선택해 주세요",
    ]),
    (579, [
        "Select the @3banished card(s)@0 to return to the @2Graveyard@0.",
        "@2墓地@0に戻す@3除外されたカード@0を選択してください。",
        "Wähle die @3verbannten Karte(n)@0, die auf den @2Friedhof@0 zurückkehren.",
        "Sélectionnez la/les @3carte(s) bannie(s)@0 à renvoyer au @2Cimetière@0.",
        "Seleziona le @3carte bandite@0 da rimandare al @2Cimitero@0.",
        "Selecciona las @3cartas desterradas@0 que regresarán al @2Cementerio@0.",
        "@2묘지@0로 되돌릴 @3제외된 카드@0를 선택해 주세요",
    ]),
];

/// The server's words for a hint in the game's language (0x1F48 lang: 0 Japanese, 1 English, 2 German, 3 French,
/// 4 Italian, 5 Spanish, 6 Korean; Chinese and any other get the English).
pub(super) fn hint_words(hint: u64, lang: u32) -> Option<&'static str> {
    let words = HINT_WORDS.iter().find(|(h, _)| *h == hint).map(|(_, w)| w)?;
    let column = match lang { 0 => 1, 2 => 2, 3 => 3, 4 => 4, 5 => 5, 6 => 6, _ => 0 };
    Some(words[column])
}


/// How many characters fit on one of the Select dialog's labels (one line; live, 57 of Pyramid Energy's 61 showed).
pub(super) const LABEL_CHARS: usize = 56;

/// `text` cut to `max` characters at a word, with "…" to say so.
pub(super) fn shortened(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max - 1).collect();
    let cut = cut.rfind(' ').filter(|&at| at > max / 2).map_or(cut.as_str(), |at| &cut[..at]);
    format!("{}…", cut.trim_end_matches([' ', ',', '.', ';']))
}

/// The client's duel-end reason (text 1999 + reason, 0x508650) for an EDOPro win reason.
pub(super) fn end_reason(reason: u64) -> u32 {
    match reason {
        0 => 12,    // "has surrendered" (12, not 5: 106 of Konami's 227 logs end on 12, none on 5)
        1 => 1,     // "Life Points have been reduced to 0"
        2 => 2,     // "has run out of cards in the Deck"
        3 => 11,    // "has ran out of time"
        4 => 14,    // "has disconnected"
        0x10 => 3,  // Exodia the Forbidden One
        0x11 => 7,  // Final Countdown
        0x12 => 9,  // Vennominaga the Deity of Poisonous Snakes
        0x14 => 10, // Exodius the Ultimate Forbidden Lord
        0x15 => 4,  // Destiny Board
        0x16 => 6,  // Last Turn
        _ => 8,     // "has won with" a card
    }
}

/// The panel's phase (command 5) for an ocgcore phase: 0 Draw, 1 Standby, 2 Main 1, 3 Battle (all its steps),
/// 4 Main 2, 5 End.
pub(super) fn client_phase(phase: u64) -> Option<u32> {
    Some(match phase {
        1 => 0,
        2 => 1,
        4 => 2,
        0x08..=0x80 => 3,
        0x100 => 4,
        0x200 => 5,
        _ => return None,
    })
}
