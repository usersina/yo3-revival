#!/usr/bin/env python3
"""tools/engine/added-cards.py (#75): the committed engine/added-cards.json is what the table and the pinned data build,
and the table's rules hold. Needs engine/fetch.sh's data and the game (skipped without them)."""
import importlib.util
import os
import re
import contextlib
import io
import tempfile
import unittest
from unittest import mock
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('added', Path(__file__).with_name('added-cards.py'))
added = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(added)
HAVE = os.path.exists(os.path.join(added.ENGINE, 'vendor', 'BabelCDB', 'cards.cdb')) and os.path.isdir(os.path.join(added.game_dir(), 'package'))


@unittest.skipUnless(HAVE, 'needs engine/fetch.sh and the game')
class AddedCards(unittest.TestCase):
    def test_the_committed_file_is_current(self):
        self.assertEqual(added.main(['--check']), 0)

    def test_quasar_is_the_games_kind_of_synchro(self):
        doc = added.build()
        self.assertEqual(doc['first_row'], 4834)
        quasar = doc['cards'][0]
        a, b = quasar['a'], quasar['b']
        self.assertEqual((a & 0x3FFF, (a >> 14 & 0x1FF) * 10, (a >> 23 & 0x1FF) * 10), (9709, 4000, 4000))
        f = added.fields(b)
        self.assertEqual((f['kind'], f['level'], f['art']), (18, 12, 1), 'a Synchro Effect monster, level 12, first artwork')
        self.assertNotIn('\n', quasar['desc'], "the game's line break is \\r")

    def test_an_xyz_monster_has_no_kind_the_client_shows(self):
        import sqlite3
        db = sqlite3.connect(os.path.join(added.ENGINE, 'vendor', 'BabelCDB', 'cards.cdb'))
        alias = {r[0]: r[1] for r in db.execute('select id, alias from datas')}
        data = {r[0]: {'type': r[1], 'attribute': r[2], 'race': r[3]} for r in db.execute('select id, type, attribute, race from datas')}
        learned = added.learn(added.client_rows(), data, added.passcodes(alias))
        self.assertIn(0x2021, learned['kind'], 'a Synchro Effect Monster is a kind the game shows')
        self.assertFalse([t for t in learned['kind'] if t & 0x800000], 'no Xyz kind in the client')

    def test_find_says_which_cards_fit_and_gives_their_lines(self):
        found = {name: (line, why) for name, line, why in added.find(
            ['black rose moonlight dragon', 'Number 39: Utopia', 'Stardust Dragon', 'Shooting Quasar Dragon', 'Nope Nope'])}
        line, why = found['Black Rose Moonlight Dragon']
        nxt = str(added.FIRST_ROW + len(added.table()))
        self.assertEqual((line.split('\t')[:7], why), ([nxt, '11123', '', '', '', '', 'Black Rose Moonlight Dragon'], []), 'the next row, its Konami id')
        self.assertIn("can't show it", found['Number 39: Utopia'][1][0])
        self.assertEqual(found['Stardust Dragon'][1], ['the game already has it'])
        self.assertEqual(found['Shooting Quasar Dragon'][1], ['engine/added/cards.tsv already has it'])
        self.assertIn('check the spelling', found['Nope Nope'][1][0])

    def test_spells_and_traps_are_the_games_kinds(self):
        ctx = added.load()
        st = ctx['learned']['spell_trap']
        # (kind, attribute, race, subtype): 13 Spell / 14 Trap with attribute 8/9 and race 23/24, as every game card has.
        self.assertEqual(st[0x2], (13, 8, 23, 0))
        self.assertEqual(st[0x80002], (13, 8, 23, 2), 'Field')
        self.assertEqual(st[0x10002], (13, 8, 23, 5), 'Quick-Play')
        self.assertEqual(st[0x100004], (14, 9, 24, 1), 'Counter Trap')
        advanced_dark = ctx['data'][12644061]
        a, b, why = added.encode(advanced_dark, 10214, ctx['learned'])
        f = added.fields(b)
        self.assertEqual((why, a, f['kind'], f['attribute'], f['race'], f['subtype'], f['level']), ([], 10214, 13, 8, 23, 2, 0))

    def test_a_newer_card_gets_a_stand_in_id_the_client_can_hold(self):
        ctx = added.load()
        (name, line, why), = added.find(['15443125'], ctx)  # Spright Starter, Konami id 17457
        self.assertEqual((name, why), ('Spright Starter', []))
        _, cid, sid, _, _, _, _, _ = line.split('\t')
        sid = int(sid)
        self.assertEqual(cid, '17457')
        self.assertTrue(ctx['base'] <= sid <= 0x3FFF and sid not in ctx['in_game'] and sid not in ctx['konami'])
        self.assertEqual(added.find(['cid:17457'], ctx)[0][0], name, 'by Konami id too')
        self.assertEqual(added.find(['Spright Starter', 'Steel-Stringed Sacrifice'], ctx)[1][1].split('\t')[2],
                         str(added.next_stand_in(ctx, {sid} | {int(l['stand_in']) for l in added.table() if l['stand_in']})), 'each card of a batch its own stand-in')
        # The table takes it: the client's id is the stand-in, the real one is kept.
        with tempfile.TemporaryDirectory() as tmp:
            table = os.path.join(tmp, 'cards.tsv')
            with open(added.TABLE) as f:
                text = f.read()
            with open(table, 'w') as f:
                f.write(text + line + 'test\n')
            with mock.patch.object(added, 'TABLE', table):
                card = added.build()['cards'][-1]
            self.assertEqual((card['cid'], card['konami_id'], card['a'] & 0x3FFF, card['passcode']), (sid, 17457, sid, 15443125))
            for wrong, says in ((line.replace(f'\t{sid}\t', '\t\t'), 'give it a stand-in'),
                                (line.replace(f'\t{sid}\t', '\t3900\t'), 'is a card of the game'),
                                (line.replace(f'\t{sid}\t', '\t9709\t'), "already a card's Konami id")):
                with open(table, 'w') as f:
                    f.write(text + wrong + 'test\n')
                with mock.patch.object(added, 'TABLE', table), self.assertRaises(SystemExit) as e:
                    added.build()
                self.assertIn(says, str(e.exception))

    def test_a_newer_card_is_refused_when_no_stand_in_is_left(self):
        # About 1,200 ids are free: when they run out, --find says so instead of failing.
        ctx = added.load()
        with mock.patch.object(added, 'stand_in_problem', lambda *a: 'taken'):
            (name, line, why), = added.find(['18847598'], ctx)
        self.assertIsNone(line)
        self.assertTrue(any('no stand-in id is left' in w for w in why), why)

    def test_genre_bits_follow_the_games_own_cards(self):
        # Edit Deck's search by effect: the learned rules mark the game's cards as the game does, nearly always.
        ctx = added.load()
        tp = fp = 0
        for (a, _), genre in zip(ctx['rows'], ctx['genres']):
            code = ctx['passcode'].get(a & 0x3FFF)
            if a & 0x3FFF and code in ctx['data']:
                guess = added.genre_of(added.features(ctx['data'][code], code), ctx['genre'])
                tp += (guess & genre).bit_count()
                fp += (guess & ~genre).bit_count()
        self.assertGreater(tp / (tp + fp), 0.93, 'the bits an added card gets are the game\'s for such a card')
        self.assertIn(22, ctx['genre'], 'Flip Effect (texts 1101 + bit)')
        quasar = added.build()['cards'][0]['genre']
        for bit, what in ((3, 'Special Summon'), (18, 'Multiple Attack'), (10, 'Destroy monster')):
            self.assertTrue(quasar >> bit & 1, f'Shooting Quasar Dragon is found under {what}')

    def test_a_rarity_sets_the_reveals_glow_bits(self):
        ctx = added.load()
        (_, line, _), = added.find(['Black Rose Moonlight Dragon'], ctx)
        with tempfile.TemporaryDirectory() as tmp:
            table = os.path.join(tmp, 'cards.tsv')
            with open(added.TABLE) as f:
                text = f.read()
            def with_rarity(rarity):
                parts = line.split('\t')
                parts[3] = rarity
                return '\t'.join(parts)
            for rarity, bits in (('', 0), ('ultra', 2), ('Rare', 4)):
                with open(table, 'w') as f:
                    f.write(text + with_rarity(rarity) + 'test\n')
                with mock.patch.object(added, 'TABLE', table):
                    self.assertEqual(added.build()['cards'][-1]['b'] >> 24 & 7, bits, rarity)
            with open(table, 'w') as f:
                f.write(text + with_rarity('shiny') + 'test\n')
            with mock.patch.object(added, 'TABLE', table), self.assertRaises(SystemExit):
                added.build()

    def test_an_anime_only_card_is_marked_and_tagged(self):
        # Orichalcos Gigas (#75's first anime-only card): no Konami id, a stand-in, the unofficial database's data, the
        # [Anime] line first, genre bits from its script (it Special Summons itself and gains ATK).
        gigas = next(c for c in added.build()['cards'] if c['name'] == 'Orichalcos Gigas')
        self.assertEqual((gigas['anime_only'], gigas['passcode'], gigas['cid']), (True, 170000171, 3968))
        self.assertTrue(gigas['desc'].startswith('[Anime]\r'))
        self.assertEqual((gigas['a'] >> 14 & 0x1FF, gigas['a'] >> 23 & 0x1FF, added.fields(gigas['b'])['level']), (40, 150, 4))
        self.assertTrue(gigas['genre'] >> 3 & 1 and gigas['genre'] >> 7 & 1, 'Special Summon, Increase/Decrease ATK/DEF')
        ctx = added.load()
        found = {n: (l, w) for n, l, w in added.find(['Orichalcos Gigas', 'anime:170000172', 'Orichalcos Shunoros (Anime)'], ctx)}
        self.assertEqual(found['Orichalcos Gigas'][1], ['engine/added/cards.tsv already has it'])
        line = found['Orichalcos Malevolence'][0].split('\t')
        self.assertEqual((line[1], line[4]), ('', '170000172'), 'no Konami id, its anime passcode')
        self.assertIn('check the spelling', found['Orichalcos Shunoros (Anime)'][1][0], 'an anime version of a printed card is no anime-only card')
        with tempfile.TemporaryDirectory() as tmp:
            table = os.path.join(tmp, 'cards.tsv')
            with open(added.TABLE) as f:
                text = f.read()
            with open(table, 'w') as f:
                f.write(re.sub(r'\t\t3968\t(\w*)\t170000171', r'\t4007\t3968\t\1\t170000171', text))
            with mock.patch.object(added, 'TABLE', table), self.assertRaises(SystemExit) as e:
                added.build()
            self.assertIn('has no Konami id', str(e.exception))

    def test_an_anime_version_of_a_printed_card_is_no_anime_only_card_to_add(self):
        # Rainbow Bridge Bifrost (id 5115) plays 100000539 as its anime version: adding that passcode again as an
        # anime-only card would give the engine two client ids for one passcode (batch 4's fuzz found the drift).
        found = {n: (l, w) for n, l, w in added.find(['anime:100000539'], added.load())}
        (line, why), = found.values()
        self.assertIsNone(line)
        self.assertTrue(any('already the anime version of Rainbow Bridge Bifrost' in w for w in why), why)

    def test_find_gives_the_anime_table_line_and_what_the_anime_map_would_make_of_it(self):
        # A printed card needs a line in engine/anime/cards.tsv too: --find writes it and runs the anime map's own rules
        # on it. Black Rose Moonlight Dragon's anime version is mapped; an anime-only card gets no line.
        ctx = added.load()
        results = added.find(['Black Rose Moonlight Dragon', 'anime:170000172'], ctx)
        rows = added.anime_rows(results, ctx)
        self.assertEqual([r['line'] for r in rows], ['11123\tBlack Rose Moonlight Dragon\t33698022\t511002831\tAnime\tadded card (#75)'])
        self.assertEqual(added.anime_outcomes(rows)[11123][0], 'mapped')


    def test_find_notes_mechanics_the_game_never_had(self):
        # The client can show a Spell, but Pendulum Call works on Pendulum Zones, Xyz Reborn brings back an Xyz Monster
        # and Link Restart a Link Monster: --find names those lines to read. Black Rose Moonlight Dragon and Ash Blossom
        # mention none.
        ctx = added.load()
        notes = {name: added.notes(line, ctx) for name, line, _ in added.find(
            ['Pendulum Call', 'Xyz Reborn', 'Link Restart', 'Black Rose Moonlight Dragon', 'Ash Blossom & Joyous Spring'], ctx)}
        self.assertTrue(any('Pendulum' in n for n in notes['Pendulum Call']), notes)
        self.assertTrue(any('Xyz' in n for n in notes['Xyz Reborn']), notes)
        self.assertTrue(any('Link' in n for n in notes['Link Restart']), notes)
        self.assertEqual((notes['Black Rose Moonlight Dragon'], notes['Ash Blossom & Joyous Spring']), ([], []))

    def test_find_points_out_what_plays_differently_at_infinity(self):
        # Without holding a card back, --find points out a script that sets LP (Super All In!), works an amount out
        # from LP and sets it (Lucky Punch's half), for a case at ∞; Ash Blossom touches neither and gets none. The
        # approved line and the blocking notes are unchanged.
        ctx = added.load()
        found = {name: (line, added.infinity_hints(line, ctx), added.notes(line, ctx)) for name, line, _ in added.find(
            ['Super All In!', 'Lucky Punch', 'Ash Blossom & Joyous Spring'], ctx)}
        self.assertTrue(all(line for line, _, _ in found.values()), found)
        self.assertTrue(any(h.startswith('sets LP') for h in found['Super All In!'][1]), found)
        self.assertTrue({'sets LP', 'works an amount out from LP'} <= {h.split(':')[0] for h in found['Lucky Punch'][1]}, found)
        self.assertTrue(any('infinite_lp.rs' in h for h in found['Lucky Punch'][1]), found)
        self.assertEqual(found['Ash Blossom & Joyous Spring'][1], [])
        self.assertEqual([n for _, _, n in found.values()], [[], [], []])

    def test_a_token_is_a_row_and_a_copy_token_stands_in_with_fiend_dark_level_1(self):
        # A Token has no Konami id: --find takes its name or token:PASSCODE and gives it a stand-in; the build marks it a
        # Token (kind 10, the game's Tokens' kind) and, when only the anime database has it, anime-only.
        ctx = added.load()
        self.assertEqual(added.made_tokens(9047460, ctx), [9047461], "Steam the Cloak's Steam Token (its passcode + 1)")
        # A Token that copies a monster (Spirit Illusion's Doppelganger Token) has no Type, Attribute or Level of its own:
        # its row is a Fiend, DARK, Level 1 one, which the engine's field report replaces as it is summoned.
        copy = {'type': added.TYPE_TOKEN | 0x11, 'attribute': 0, 'race': 0, 'level': 0, 'atk': -2, 'def': -2}
        plain = dict(copy, attribute=0x20, race=0x8, level=1)
        self.assertEqual(added.encode(copy, 6014, ctx['learned']), added.encode(plain, 6014, ctx['learned']))
        self.assertEqual(added.encode(copy, 6014, ctx['learned'])[2], [])
        # Spirit Illusion's Doppelganger Token is a row now: the card is found.
        (name, line, why), = added.find(['Spirit Illusion'], ctx)
        self.assertFalse(any('Doppelganger Token' in w for w in why), why)
        built = {c['name']: c for c in added.build()['cards']}
        steam, gadget = built['Steam Token'], built['Gadget Token']
        self.assertEqual((steam.get('token'), steam.get('anime_only'), steam['b'] >> 3 & 0x1F), (True, None, 10))
        self.assertEqual((gadget.get('token'), gadget.get('anime_only')), (True, True), 'only the anime database has it')
        self.assertNotIn('konami_id', steam)

    def test_stand_ins_stop_at_the_reserve_and_the_count_is_reported(self):
        # The client holds ids 3900-16383; --find gives a stand-in only while more than STAND_IN_RESERVE are free, and its
        # report says how many are left.
        ctx = added.load()
        taken = {int(l['stand_in']) for l in added.table() if (l.get('stand_in') or '').strip()}
        free = len(added.free_stand_ins(ctx, taken))
        self.assertGreater(free, added.STAND_IN_RESERVE, 'the batches still have stand-ins to give')
        # A card of the anime-only batches not added yet (the batches add them one by one).
        have = {l['anime'] for l in added.table()}
        batches = os.path.join(added.ENGINE, 'added', 'anime-batches.tsv')
        with open(batches, encoding='utf-8') as f:
            waiting = [l.split('\t')[3] for l in f.read().splitlines()[1:] if l.split('\t')[3] not in have and not l.split('\t')[5].strip()]
        if not waiting:
            self.skipTest('every anime-only batch is added')
        card = f'anime:{waiting[-1]}'
        with mock.patch.object(added, 'STAND_IN_RESERVE', free):
            (name, line, why), = added.find([card], ctx)
            self.assertIsNone(line)
            self.assertIn(added.NO_STAND_IN.split(':')[0], '; '.join(why))
        with mock.patch.object(added, 'STAND_IN_RESERVE', free - 1):
            (name, line, why), = added.find([card], ctx)
            self.assertTrue(line, why)
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                added.report([(name, line, why)], ctx)
            self.assertIn(f'Stand-in ids: {free - 1} free after these (0 to give, {free - 1} kept back)', out.getvalue())

    def test_add_writes_both_tables_and_rebuilds(self):
        # --add: the line with its note at the end of engine/added/cards.tsv, the anime table's line in id order, then the
        # three generators. A card that can't be added, or one with notes not yet read (--checked), writes nothing.
        with tempfile.TemporaryDirectory() as tmp:
            table, anime = os.path.join(tmp, 'cards.tsv'), os.path.join(tmp, 'anime.tsv')
            for src, dst in ((added.TABLE, table), (added.ANIME_TABLE, anime)):
                with open(src, encoding='utf-8') as a, open(dst, 'w', encoding='utf-8') as b:
                    b.write(a.read())
            before = [open(table).read(), open(anime).read()]
            ran = []
            with mock.patch.object(added, 'TABLE', table), mock.patch.object(added, 'ANIME_TABLE', anime), \
                    mock.patch.object(added, 'regenerate', lambda: ran.append(1) or 0):
                self.assertEqual(added.main(['--add', 'Black Rose Moonlight Dragon', 'Number 39: Utopia', '--note', 'test']), 1)
                self.assertEqual(added.main(['--add', 'Pendulum Call', '--note', 'test']), 1)
                self.assertEqual(([open(table).read(), open(anime).read()], ran), (before, []), 'nothing written')
                self.assertEqual(added.main(['--add', 'Black Rose Moonlight Dragon', '--note', 'asked by test']), 0)
            last = open(table).read().rstrip('\n').split('\n')[-1].split('\t')
            self.assertEqual((last[1], last[6], last[7]), ('11123', 'Black Rose Moonlight Dragon', 'asked by test'))
            ids = [int(l.split('\t')[0]) for l in open(anime).read().rstrip('\n').split('\n')[1:]]
            self.assertIn(11123, ids)
            self.assertEqual(ids, sorted(ids), 'the anime table stays in id order')
            self.assertEqual(ran, [1])

class Pure(unittest.TestCase):
    def test_the_databases_unofficial_text_note_is_left_out(self):
        desc = 'Cannot attack.\r\n\r\n* The above text is unofficial and describes the card\'s functionality in the OCG.'
        self.assertEqual(added.card_text(desc), 'Cannot attack.')
        self.assertEqual(added.card_text('Line one.\nLine two.'), 'Line one.\rLine two.')

    def test_fields_read_the_rows_second_word(self):
        f = added.fields(18 << 3 | 1 << 8 | 12 << 12 | 1 << 19 | 1 << 27)
        self.assertEqual((f['kind'], f['attribute'], f['level'], f['race'], f['art']), (18, 1, 12, 1, 1))


    def test_mechanics_name_their_lines_and_leave_material_rules_alone(self):
        lua = 'local s,id=GetID()\nfunction s.filter(c) return c:IsType(TYPE_XYZ) end\n' \
              'e1:SetCode(EFFECT_CANNOT_BE_XYZ_MATERIAL)\nDuel.GetFieldGroup(tp,LOCATION_PZONE,0)\n'
        self.assertEqual(added.mechanics(lua), [('Xyz', [2]), ('Pendulum', [4])])
        self.assertEqual(added.mechanics('e1:SetCode(EFFECT_CANNOT_BE_LINK_MATERIAL)\n'), [])

if __name__ == '__main__':
    unittest.main()
