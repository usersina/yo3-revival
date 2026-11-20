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
TABLE = [line.split('\t') for line in (ROOT / 'engine/anime/research.tsv').read_text(encoding='utf-8').splitlines()[1:]]


class AnimeMap(unittest.TestCase):
    def test_every_researched_card_has_one_recorded_disposition(self):
        self.assertEqual(sorted(int(r[0]) for r in TABLE), sorted(CARDS))
        self.assertEqual(len(TABLE), len(CARDS), 'a Konami id twice')
        counts = {s: sum(1 for c in CARDS.values() if c['status'] == s) for s in ('mapped', 'default', 'held')}
        self.assertEqual(counts, MAP['counts'])
        self.assertEqual(sum(counts.values()), 418, '415 candidates and the 3 cards that needed custom work')
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
        self.assertEqual(sorted(recs), [8035, 8302, 8538, 9093])
        upstream = {8538: 513000096, 9093: 511002602}
        for cid, code in upstream.items():
            c, r = CARDS[cid], recs[cid]
            self.assertEqual((c['status'], c['script'], c.get('upstream_record')), ('mapped', 'unofficial/c%d.lua' % code, True))
            self.assertEqual((r['atk'], r['alias']), (-2, r['official']), 'the game\'s ? kept, the official card as alias')
            self.assertFalse(r['reconstruction'])
            self.assertNotIn('differs', c)
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

    def test_the_egyptian_gods_are_mapped_and_skill_drain_is_ours_but_unchanged_elsewhere(self):
        for cid, anime in ((4998, 511600398), (4999, 511600399), (5000, 511600400)):
            self.assertEqual((CARDS[cid]['status'], CARDS[cid]['anime']), ('mapped', anime))
        text = (ROOT / 'engine/overrides/c82732705.lua').read_text(encoding='utf-8')
        for anime in (511600398, 511600399, 511600400):
            self.assertIn(str(anime), text, 'the override names the anime Gods by their passcodes only')
        for official in (10000000, 10000010, 10000020):
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


if __name__ == '__main__':
    unittest.main()
