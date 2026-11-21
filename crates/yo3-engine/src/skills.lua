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

-- Destiny Draw (issue #71), the host's option for a casual duel against the computer: once per duel, at the start of
-- the player's Draw Phase with LP at or below 1000, the player may pick any card left in the Deck as the card drawn
-- this turn. Declined, it is offered again at a later draw while unused. The two texts are the engine's own strings
-- (cards.rs `skill_string`), worded by the server through the connection DLL.
YO3Skill.DESTINY_ASK=0x7FF00000<<20
YO3Skill.DESTINY_PICK=(0x7FF00000<<20)|1
function YO3Skill.DestinyDraw(p)
	local used=false
	local e=Effect.GlobalEffect()
	e:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
	e:SetCode(EVENT_PREDRAW)
	e:SetCondition(function()
		return not used and Duel.IsTurnPlayer(p) and Duel.GetDrawCount(p)>0 and Duel.GetLP(p)<=1000
			and Duel.GetFieldGroupCount(p,LOCATION_DECK,0)>0
	end)
	e:SetOperation(function()
		if not Duel.SelectYesNo(p,YO3Skill.DESTINY_ASK) then return end
		local names=Group.CreateGroup()
		for tc in aux.Next(Duel.GetFieldGroup(p,LOCATION_DECK,0)) do
			if not names:IsExists(Card.IsCode,1,nil,tc:GetCode()) then names:AddCard(tc) end
		end
		Duel.Hint(HINT_SELECTMSG,p,YO3Skill.DESTINY_PICK)
		local tc=names:Select(p,1,1,nil):GetFirst()
		if tc then
			used=true
			Duel.MoveToDeckTop(tc)
		end
	end)
	Duel.RegisterEffect(e,p)
end
