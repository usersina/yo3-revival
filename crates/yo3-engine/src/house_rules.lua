-- YO3 Revival's house rules, loaded after utility.lua into every duel and the bot's look-ahead copies
-- (docs/rules-privacy.md, "House rules"). YO3_REVIVAL_RULE is the `revival-rule` setting, set just before this file:
--
--   official                 the official ruling: nothing here changes the game.
--   ignoring-means-all       an effect that Special Summons "ignoring its Summoning conditions" (the core's `nocheck`)
--                            also ignores "must first be properly Summoned" (the revive limit, the core's `nolimit`):
--                            Illusion Gate takes a Divine Serpent Geh that was only ever sent to the Graveyard.
--   graveyard-is-graveyard   that too, and any Special Summon of a card in the Graveyard or banished ignores the revive
--                            limit: Monster Reborn revives a Ritual Monster that was discarded. A card's own written
--                            conditions still apply (no `nocheck`), and the hand and the Deck keep the limit.
local rule = YO3_REVIVAL_RULE or "official"
if rule ~= "official" then
	local anywhere = rule == "graveyard-is-graveyard"
	local away = LOCATION_GRAVE | LOCATION_REMOVED
	-- Whether `c` (a Card or a Group) is summoned past the revive limit: with `nocheck`, or from the Graveyard or
	-- banished under graveyard-is-graveyard. A Group lifts it only when none of its cards is limited elsewhere.
	local function lifted(c, nocheck, nolimit)
		if nolimit or nocheck then return nolimit or nocheck end
		if not anywhere or not c then return nolimit end
		if not c.IsExists then return c:IsLocation(away) or nolimit end -- a Card (a Group has IsExists)
		local limited_elsewhere = c:IsExists(function(tc)
			return not tc:IsLocation(away) and tc:IsHasEffect(EFFECT_REVIVE_LIMIT) and not tc:IsStatus(STATUS_PROC_COMPLETE)
		end, 1, nil)
		return not limited_elsewhere or nolimit
	end
	local can_be = Card.IsCanBeSpecialSummoned
	Card.IsCanBeSpecialSummoned = function(c, e, sumtype, sumplayer, nocheck, nolimit, ...)
		return can_be(c, e, sumtype, sumplayer, nocheck, lifted(c, nocheck, nolimit), ...)
	end
	local step = Duel.SpecialSummonStep
	Duel.SpecialSummonStep = function(c, sumtype, sumplayer, target, nocheck, nolimit, ...)
		return step(c, sumtype, sumplayer, target, nocheck, lifted(c, nocheck, nolimit), ...)
	end
	local summon = Duel.SpecialSummon
	Duel.SpecialSummon = function(g, sumtype, sumplayer, target, nocheck, nolimit, ...)
		return summon(g, sumtype, sumplayer, target, nocheck, lifted(g, nocheck, nolimit), ...)
	end
end
