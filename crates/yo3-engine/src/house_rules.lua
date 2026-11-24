-- YO3 Revival's house rules for every duel, loaded after utility.lua (docs/rules-privacy.md, "House rules").
--
-- "Ignoring its Summoning conditions" means all of them: an effect that Special Summons a monster ignoring its
-- Summoning conditions (the core's `nocheck`) also ignores "must first be properly Summoned" (the revive limit, the
-- core's `nolimit`). So Illusion Gate takes a Divine Serpent Geh that was only ever sent to the Graveyard, or a Synchro
-- Monster that was never Synchro Summoned. The official ruling keeps the revive limit; the anime never did. Every other
-- Special Summon (Monster Reborn, Call of the Haunted...) passes no `nocheck` and keeps every limit.
do
	local can_be = Card.IsCanBeSpecialSummoned
	Card.IsCanBeSpecialSummoned = function(c, e, sumtype, sumplayer, nocheck, nolimit, ...)
		return can_be(c, e, sumtype, sumplayer, nocheck, nolimit or nocheck, ...)
	end
	local step = Duel.SpecialSummonStep
	Duel.SpecialSummonStep = function(c, sumtype, sumplayer, target, nocheck, nolimit, ...)
		return step(c, sumtype, sumplayer, target, nocheck, nolimit or nocheck, ...)
	end
	local summon = Duel.SpecialSummon
	Duel.SpecialSummon = function(g, sumtype, sumplayer, target, nocheck, nolimit, ...)
		return summon(g, sumtype, sumplayer, target, nocheck, nolimit or nocheck, ...)
	end
end
