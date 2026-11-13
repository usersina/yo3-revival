-- yo3-revival's skills: hidden powers the server gives an NPC (crates/yo3-server/src/npc.rs), loaded into the duel
-- for that player before it starts. No card shows them: they are global effects.
YO3Skill={}

-- The Heart of the Cards: before each of the player's draws, it picks which card of its Deck is on top (one of each
-- name), so it draws the card it needs. The duel bot picks by playing each candidate ahead. Moving a card within the
-- Deck sends no message: the client just sees the draw.
function YO3Skill.HeartOfTheCards(p)
	local e=Effect.GlobalEffect()
	e:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
	e:SetCode(EVENT_PREDRAW)
	e:SetCondition(function()
		return Duel.IsTurnPlayer(p) and Duel.GetDrawCount(p)>0 and Duel.GetFieldGroupCount(p,LOCATION_DECK,0)>1
	end)
	e:SetOperation(function()
		local names=Group.CreateGroup()
		for tc in aux.Next(Duel.GetFieldGroup(p,LOCATION_DECK,0)) do
			if not names:IsExists(Card.IsCode,1,nil,tc:GetCode()) then names:AddCard(tc) end
		end
		Duel.Hint(HINT_SELECTMSG,p,HINTMSG_ATOHAND)
		local tc=names:Select(p,1,1,nil):GetFirst()
		if tc then Duel.MoveToDeckTop(tc) end
	end)
	Duel.RegisterEffect(e,p)
end
