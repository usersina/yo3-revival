#!/usr/bin/env python3
"""tools/engine/anime-map.py and engine/anime-cards.json (issue #60): the reviewed map never follows an upstream alias to
another card, holds back what the client can't show, keeps the cards that need custom work out, and is what the table
builds (that last check needs engine/fetch.sh and is skipped without it)."""
import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('anime_map', Path(__file__).with_name('anime-map.py'))
anime_map = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(anime_map)

MAP = json.loads((ROOT / 'engine/anime-cards.json').read_text(encoding='utf-8'))
CARDS = {c['cid']: c for c in MAP['cards']}
ROWS = [dict(zip(anime_map.COLUMNS, (line.split('\t') + [''] * 6)[:6]))
        for line in (ROOT / 'engine/anime/cards.tsv').read_text(encoding='utf-8').splitlines()[1:]]
# The reviewed cards: those with an anime version, and the one held for research.
TABLE = [[r['cid'], r['name'], r['anime_id'], r['status'], r['note']] for r in ROWS if r['status'] in anime_map.WITH_ANIME | {'Held'}]


class AnimeMap(unittest.TestCase):
    def test_the_table_has_one_row_per_card_and_an_anime_id_only_with_an_anime_status(self):
        ids = [r['cid'] for r in ROWS]
        self.assertEqual(len(ids), len(set(ids)), 'a Konami id twice')
        self.assertGreater(len(ROWS), 4700, 'every card of the game has a row')
        self.assertEqual([r['cid'] for r in ROWS], sorted(ids, key=int), 'sorted by Konami id')
        for r in ROWS:
            self.assertIn(r['status'], anime_map.STATUSES, r)
            self.assertEqual(bool(r['anime_id']), r['status'] in anime_map.WITH_ANIME, r)
        by = {r['name']: r for r in ROWS}
        self.assertEqual((by['The Winged Dragon of Ra']['anime_id'], by['The Winged Dragon of Ra']['edopro_id']), ('513000134', '10000010'))
        self.assertEqual(by['Deck Devastation Virus']['anime_id'], '', 'its anime appearance is the game card; 511001119 is another card')


    def test_every_researched_card_has_one_recorded_disposition(self):
        self.assertEqual(sorted(int(r[0]) for r in TABLE), sorted(CARDS))
        self.assertEqual(len(TABLE), len(CARDS), 'a Konami id twice')
        counts = {s: sum(1 for c in CARDS.values() if c['status'] == s) for s in ('mapped', 'default', 'held')}
        self.assertEqual(counts, MAP['counts'])
        self.assertEqual(sum(counts.values()), 666, '415 candidates, the 3 cards that needed custom work, the added cards with anime versions (#75: Shooting Quasar Dragon, Divine Serpent Geh, Underworld Circle), the three Wicked Gods (their manga versions) and 242 of the printed cards added from the anime (#75, 2026-11-25 and 26)')
        for c in CARDS.values():
            self.assertEqual(bool(c.get('holds')), c['status'] == 'held', c['name'])

    def test_an_alias_is_never_followed_to_another_card(self):
        for c in CARDS.values():
            if c['status'] == 'mapped':
                if c['cid'] not in anime_map.ALIAS_KEPT:
                    self.assertIn(c['alias'], (0, c['official']), '%s aliases another card' % c['name'])
        # Anime Power Bond aliases Polymerization on purpose ("treated as Polymerization"): kept by name, with its identity
        # tested (tests/anime_cards.rs); an alias to another card holds any other card.
        power_bond = CARDS[6398]
        self.assertEqual((power_bond['status'], power_bond['alias']), ('mapped', 24094653))
        self.assertEqual(list(anime_map.ALIAS_KEPT), [6398])

    def test_mapped_cards_are_ones_the_client_can_show(self):
        # Flame Swordsman is a Fusion Monster in the client and a Normal Monster in the anime: the card profile shows it as
        # the anime's Main Deck Normal Monster (a proof card), and Five-Headed Dragon (a Fusion in the game, a Ritual in the
        # anime) as a Main Deck Ritual Monster; Thousand Dragon keeps the game's version (retained with Time Wizard).
        flame = CARDS[4021]
        self.assertEqual(flame['status'], 'mapped')
        self.assertTrue(any(h.startswith('deck:') for h in flame['profile']))
        self.assertEqual(CARDS[5502]['status'], 'mapped')
        self.assertTrue(any(h.startswith('deck:') for h in CARDS[5502]['profile']))
        self.assertEqual(CARDS[4075]['status'], 'default')
        self.assertIn('deck', CARDS[4075]['differs'])
        for c in CARDS.values():
            if c['status'] == 'mapped':
                self.assertTrue(c['anime'] and c['official'] and c['script'] or c['label'] == 'Anime / Normal', c['name'])
        # The reviewed examples of the issue play as their anime versions.
        for cid, anime in ((7696, 511009416), (5479, 511001126), (4887, 513000001)):
            self.assertEqual((CARDS[cid]['status'], CARDS[cid]['anime']), ('mapped', anime))

    def test_placeholder_cards_have_our_own_record_and_script_and_the_research_card_is_held(self):
        # Dark Strike Fighter and Fortune Lady Fire: ProjectIgnis has only a placeholder, so the anime data record
        # (engine/anime/records.json) and the script (engine/overrides/) are ours; the game's cid and the official card stay.
        recs = {r['cid']: r for r in MAP['records']}
        # Clear Vice Dragon, Blackwing - Aurora and anime Divine Serpent Geh (an added card, #75) keep the game's "?" ATK.
        self.assertEqual(sorted(recs), [8035, 8302, 8538, 9093, 10326, 11152, 11885, 11892, 12671, 13803, 13804, 13805, 13807])
        # Pumpkin Carriage, Iron Hans, Iron Knight and Hexe Trude (added cards, #75): the pinned database marks their anime
        # rows Tokens (0x4021), so a copy sent to the hand would vanish; the record is the row as an Effect Monster.
        tokens = {13803: 511000432, 13804: 511001416, 13805: 511001415, 13807: 511000434}
        for cid, code in tokens.items():
            c, r = CARDS[cid], recs.pop(cid)
            self.assertEqual((c['status'], c['script'], c.get('upstream_record')), ('mapped', 'unofficial/c%d.lua' % code, True))
            self.assertEqual((r['type'], r['alias'], r['reconstruction']), (0x21, r['official'], False))
        # The Eye of Timaeus, The Fang of Critias, The Claw of Hermos and Legend of Heart (added cards): the anime rows have no
        # type at all, so the record gives each the official card's, a Normal Spell (the Legendary Dragons' archetype too).
        spells = {10326: 170000152, 11885: 170000151, 11892: 170000153, 11152: 170000201}
        for cid, code in spells.items():
            c, r = CARDS[cid], recs.pop(cid)
            self.assertEqual((c['status'], c['script'], c.get('upstream_record'), c.get('differs')), ('mapped', 'unofficial/c%d.lua' % code, True, None))
            self.assertEqual((r['type'], r['setcode'], r['alias']), (0x2, 0 if cid == 11152 else 161, r['official']))
        upstream = {8538: 513000096, 9093: 511002602, 12671: 170000170}
        for cid, code in upstream.items():
            c, r = CARDS[cid], recs[cid]
            self.assertEqual((c['status'], c['script'], c.get('upstream_record')), ('mapped', 'unofficial/c%d.lua' % code, True))
            self.assertEqual((r['atk'], r['alias']), (-2, r['official']), 'the game\'s ? kept, the official card as alias')
            self.assertFalse(r['reconstruction'])
            self.assertEqual(c.get('differs'), ['level'] if cid == 12671 else None, 'only anime Geh differs: Level 10, not 12')
        recs = {k: v for k, v in recs.items() if k not in upstream}
        for cid, (code, stats) in {8035: (513000079, (2600, 1800, 7)), 8302: (511247002, (-2, -2, 2))}.items():
            c, r = CARDS[cid], recs[cid]
            self.assertEqual((c['status'], c['anime'], c['script']), ('mapped', code, 'overrides/c%d.lua' % code))
            self.assertTrue(c['reconstruction'] and r['reconstruction'])
            self.assertEqual((r['atk'], r['def'], r['level']), stats)
            self.assertEqual(r['alias'], r['official'], 'kept the official card as its alias')
            self.assertEqual(r['official'], c['official'])
            self.assertTrue((ROOT / ('engine/overrides/c%d.lua' % code)).is_file())
            self.assertIn('reconstruction', r['text'].lower(), 'the card says in its own text that it is a reconstruction')
            self.assertNotIn('differs', c, 'the profile has nothing to show: the stats are the official card\'s')
        self.assertEqual(recs[8035]['type'], 0x2021, 'a Synchro Effect Monster')
        # Temple of the Kings would need a reconstruction first: the game's version is retained, nothing installed.
        temple = CARDS[5231]
        self.assertEqual((temple['status'], temple['anime']), ('default', None))
        self.assertIn('reconstruction', temple['reason'])
        self.assertIn('episodes 87 and 89', temple['reason'])
        # Negate Attack is a proof card: the pinned database's version (a Quick-Play Spell), shown by the profile.
        self.assertEqual((CARDS[4804]['status'], CARDS[4804]['anime']), ('mapped', 511004341))
        self.assertTrue(CARDS[4804]['profile'][0].startswith('kind:'))

    def test_the_gods_play_their_manga_versions_and_skill_drain_is_ours_but_unchanged_elsewhere(self):
        manga = ((4998, 513000135), (4999, 513000136), (5000, 513000134), (6405, 513000137), (7091, 513000138), (7092, 513000139))
        for cid, anime in manga:
            self.assertEqual((CARDS[cid]['status'], CARDS[cid]['anime'], CARDS[cid]['label']), ('mapped', anime, 'Manga'))
        text = (ROOT / 'engine/overrides/c82732705.lua').read_text(encoding='utf-8')
        for _, anime in manga:
            self.assertIn(str(anime), text, 'the override names the manga Gods by their passcodes only')
        for official in (10000000, 10000010, 10000020, 62180201, 21208154, 57793869):
            self.assertNotIn(str(official), text, 'the official Gods are not touched')

    def test_every_reviewed_card_ends_with_one_of_three_dispositions(self):
        # anime enabled, the game's version retained with a reason, or held for a technical fix (#60's closing plan).
        self.assertEqual({c['status'] for c in CARDS.values()}, {'mapped', 'default'}, 'nothing is held now: every reviewed card is enabled or retained on purpose')
        for c in CARDS.values():
            if c['status'] == 'default':
                self.assertTrue(c['reason'].startswith('Default retained'), c['name'])
                self.assertNotIn('holds', c)
            if c['status'] == 'held':
                self.assertTrue(c['holds'], c['name'])
        # Time Wizard and Thousand Dragon are reviewed together and retained together.
        self.assertEqual((CARDS[4022]['status'], CARDS[4075]['status']), ('default', 'default'))
        self.assertEqual(MAP['counts'], {s: sum(1 for c in CARDS.values() if c['status'] == s) for s in ('mapped', 'default', 'held')})

    def test_a_script_that_names_a_held_anime_card_is_held_too(self):
        by_anime = {c['anime']: c for c in CARDS.values() if c['anime']}
        for c in CARDS.values():
            if c['status'] == 'mapped':
                for dep in c.get('depends', []):
                    self.assertNotEqual(by_anime.get(dep, {'status': 'mapped'})['status'], 'held', c['name'])

    def test_stats_must_be_the_same_number_even_for_a_question_mark(self):
        self.assertFalse(anime_map.stat_differs(1400, 1400))
        self.assertFalse(anime_map.stat_differs(-2, -2))
        self.assertTrue(anime_map.stat_differs(-2, 0), 'the client shows its own number for "?"')
        self.assertTrue(anime_map.stat_differs(0, -2))
        self.assertTrue(anime_map.stat_differs(1400, 2900))

    def test_differences_name_what_the_client_would_show_wrongly(self):
        fusion, normal = 0x1 | 0x40 | 0x20, 0x1 | 0x10
        categories = lambda o, a: [c for c, _ in anime_map.differences(o, a)]
        self.assertEqual(categories((fusion, 1800, 1600, 5, 1, 4), (normal, 1800, 1600, 5, 1, 4)), ['deck', 'type'])
        self.assertEqual(categories((0x4, 0, 0, 0, 0, 0), (0x2, 0, 0, 0, 0, 0)), ['kind'])
        self.assertEqual(categories((0x21, 100, 200, 4, 1, 4), (0x21, 100, 200, 4, 1, 4)), [])
        self.assertEqual(anime_map.differences((0x21, 100, 200, 4, 1, 4), (0x21, 100, 200, 4, 2, 4)), [('race', 'race: 1 -> 2')])
        # Spell and Trap kinds, and the flags the client shows nowhere, are categories of their own.
        self.assertEqual(categories((0x2, 0, 0, 0, 0, 0), (0x10002, 0, 0, 0, 0, 0)), ['subtype'])
        self.assertEqual(categories((0x21, 0, 0, 4, 1, 4), (0x200021, 0, 0, 4, 1, 4)), ['flags'])

    def test_a_category_releases_cards_only_when_the_profile_covers_it_or_for_the_proof_cards(self):
        # With a category off (as it was before its release), only the proof cards are covered, for the categories they need.
        was_covers = set(anime_map.PROFILE_COVERS)
        try:
            anime_map.PROFILE_COVERS.clear()
            self.assertFalse(anime_map.covered(5502, 'deck'), 'nothing is covered for a card outside the proof set')
            self.assertTrue(anime_map.covered(4021, 'deck') and anime_map.covered(4804, 'kind') and anime_map.covered(4008, 'type'))
            self.assertFalse(anime_map.covered(4021, 'atk'), 'the proof set covers its own categories only')
            anime_map.PROFILE_COVERS.add('level')
            self.assertTrue(anime_map.covered(5502, 'level') and not anime_map.covered(5502, 'race'), 'a released category covers every card')
        finally:
            anime_map.PROFILE_COVERS.clear()
            anime_map.PROFILE_COVERS.update(was_covers)
        # Proven live (card panel, trunk list, duel hover show 1/1): the flag is on.
        self.assertTrue(anime_map.EXACT_STATS_PROVEN)
        was = anime_map.EXACT_STATS_PROVEN
        try:
            anime_map.EXACT_STATS_PROVEN = True
            self.assertTrue(anime_map.covered(8461, 'atk') and anime_map.covered(8461, 'def'))
            anime_map.PROFILE_COVERS.clear()
            self.assertFalse(anime_map.covered(8461, 'level') or anime_map.covered(8461, 'race'), 'only ATK and DEF')
            self.assertFalse(anime_map.covered(5502, 'atk'), 'and only for the cards listed')
            anime_map.EXACT_STATS_PROVEN = False
            self.assertFalse(anime_map.covered(8461, 'atk'), 'held while the flag is off')
        finally:
            anime_map.EXACT_STATS_PROVEN = was
            anime_map.PROFILE_COVERS.update(was_covers)
        # Every category the generator knows is released (issue #60): each has a test card in tests/card_profile.rs and a
        # real-core scenario in tests/anime_cards.rs, and the self-play duel check passes with all of them on.
        self.assertEqual(anime_map.PROFILE_COVERS, set(anime_map.CATEGORY_ORDER))
        # So no held card is held for a category alone: what the report counts would release is nothing.
        rows = {c: (a, t) for c, a, t in anime_map.release_report(MAP)}
        self.assertEqual(list(rows), anime_map.CATEGORY_ORDER)
        self.assertTrue(all(a <= t for a, t in rows.values()))
        self.assertEqual([t for _, t in rows.values()], sorted(t for _, t in rows.values()), 'cumulative counts only grow')
        self.assertEqual([t for _, t in rows.values()][-1], 0, 'nothing is held for a category alone')

    def test_a_question_mark_the_database_spells_as_zero_is_released_by_a_corrected_record(self):
        # Clear Vice Dragon and Blackwing - Aurora: the game shows "?"; the anime database's 0 stands for it, so a bare 0 is a
        # placeholder difference the profile cannot show; their records keep the "?" (engine/anime/records.json), so no difference remains.
        self.assertEqual(anime_map.differences((0x21, -2, 0, 8, 1, 1), (0x21, 0, 0, 8, 1, 1))[0][0], 'placeholder')
        for cid in (8538, 9093):
            self.assertEqual(CARDS[cid]['status'], 'mapped')
            self.assertNotIn('holds', CARDS[cid])
        # A "?" the anime card has too, and the anime's "?" where the game has a number, are real profile differences.
        self.assertEqual(anime_map.differences((0x21, -2, 0, 8, 1, 1), (0x21, -2, 0, 8, 1, 1)), [])
        self.assertEqual(anime_map.differences((0x21, 1500, 1500, 6, 1, 16), (0x21, -2, -2, 6, 1, 16))[0][0], 'atk')

    def test_an_effect_the_deck_triggers_is_released_one_card_and_one_code_at_a_time(self):
        # A Special Summon procedure from the Deck is a route proven for the class (the Deck pile); an effect the Deck triggers is
        # released only for the cards listed with their effect code, each proven end to end (tests/anime_cards.rs `theinen_*`).
        self.assertEqual(anime_map.DECK_ROUTES, {'EFFECT_SPSUMMON_PROC'})
        self.assertEqual(anime_map.DECK_TRIGGERS, {6089: {'EVENT_LEAVE_FIELD'}})
        theinen = CARDS[6089]
        self.assertEqual((theinen['status'], theinen.get('holds')), ('mapped', None))
        self.assertEqual(CARDS[4075]['status'], 'default', 'Time Wizard stays the game card')
        self.assertEqual(CARDS[4022]['status'], 'default', 'Thousand Dragon stays the game card')
        # The same script text, for another card or another effect code, is not released.
        text = (ROOT / 'engine/vendor/CardScripts/unofficial/c513000030.lua')
        if text.exists():
            codes = anime_map.deck_effects(text.read_text(encoding='utf-8'))
            self.assertEqual(codes, ['EVENT_LEAVE_FIELD'])
            for cid, code in ((6089, 'EVENT_LEAVE_FIELD'), (1, 'EVENT_LEAVE_FIELD'), (6089, 'EVENT_PHASE+PHASE_END')):
                allowed = code in anime_map.DECK_ROUTES or code in anime_map.DECK_TRIGGERS.get(cid, ())
                self.assertEqual(allowed, (cid, code) == (6089, 'EVENT_LEAVE_FIELD'), (cid, code))

    def test_every_released_category_changes_a_mapped_card(self):
        shown = {cat for c in CARDS.values() if c['status'] == 'mapped' for cat in c.get('differs', [])}
        for category in anime_map.CATEGORY_ORDER:
            self.assertIn(category, shown, 'no mapped card differs in %s: the category is released for nothing' % category)

    def test_the_file_is_what_the_table_builds(self):
        data = anime_map.build()
        self.assertEqual((ROOT / 'engine/anime-cards.json').read_text(encoding='utf-8'), anime_map.dump(data))
        self.assertEqual((ROOT / 'docs/anime-cards.md').read_text(encoding='utf-8'), anime_map.doc(data))


    # The three helpers for a reported card (--card), an EDOPro install (--compare) and a move of the pins (--candidates).

    def test_a_card_is_looked_up_by_name_or_konami_id(self):
        ra = '\n'.join(anime_map.card_report('winged dragon of ra', MAP))
        self.assertIn('anime_id 513000134', ra)
        self.assertIn('map: mapped; script unofficial/c513000134.lua', ra)
        self.assertIn('anime text: 2096 characters', ra)
        virus = '\n'.join(anime_map.card_report('6284', MAP))
        self.assertIn('anime_id none', virus)
        self.assertIn("plays and shows as the game's card", virus)
        self.assertIn('Be more specific', anime_map.card_report('dragon', MAP)[0])
        self.assertIn('No card', anime_map.card_report('zzzz', MAP)[0])

    def test_no_anime_card_waits_outside_the_table(self):
        self.assertEqual(anime_map.candidates(), [], 'review it into engine/anime/cards.tsv, or name it in its row note')

    def test_an_edopro_install_is_compared_card_by_card(self):
        import shutil, sqlite3, tempfile
        with tempfile.TemporaryDirectory() as app:
            os_app = Path(app)
            (os_app / 'expansions').mkdir()
            (os_app / 'script/unofficial').mkdir(parents=True)
            db = os_app / 'expansions/cards-unofficial.cdb'
            shutil.copy(ROOT / 'engine/vendor/BabelCDB/cards-unofficial.cdb', db)
            with sqlite3.connect(db) as c:
                c.execute("update texts set desc = 'changed' where id = 513000009")
            shutil.copy(ROOT / 'engine/vendor/CardScripts/unofficial/c513000134.lua', os_app / 'script/unofficial/c513000134.lua')
            with open(os_app / 'script/unofficial/c513000134.lua', 'a', encoding='utf-8') as f:
                f.write('\n-- changed\n')
            report = '\n'.join(anime_map.compare_report(app))
        self.assertIn('Crush Card Virus (4667, anime 513000009): text differs', report)
        self.assertIn('The Winged Dragon of Ra (5000, anime 513000134): script differs', report)
        self.assertIn('Dark Strike Fighter (8035, anime 513000079): ours is our own record', report)


if __name__ == '__main__':
    unittest.main()
