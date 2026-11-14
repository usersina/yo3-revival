--処刑人－マキュラ
--Makyura the Destructor
local s,id=GetID()
function s.initial_effect(c)
	--activate trap in hand: "During the turn this card is sent to the Graveyard, the owner of this card can activate Trap
	--Card(s) from his/her hand": sent from anywhere, not an effect that activates, and any number of Traps (the client's
	--text; yo3-revival override: today's text wants it sent from the Monster Zone, makes it a once-per-turn effect
	--that activates and allows 1 Trap)
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_SINGLE+EFFECT_TYPE_CONTINUOUS)
	e1:SetCode(EVENT_TO_GRAVE)
	e1:SetOperation(s.operation)
	c:RegisterEffect(e1)
end
function s.operation(e,tp,eg,ep,ev,re,r,rp)
	local e1=Effect.CreateEffect(e:GetHandler())
	e1:SetType(EFFECT_TYPE_FIELD)
	e1:SetCode(EFFECT_TRAP_ACT_IN_HAND)
	e1:SetTargetRange(LOCATION_HAND,0)
	e1:SetReset(RESET_PHASE|PHASE_END)
	e1:SetDescription(aux.Stringid(id,0))
	Duel.RegisterEffect(e1,tp)
end
