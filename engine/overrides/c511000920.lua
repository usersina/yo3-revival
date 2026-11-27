--呪い移し
--Spell Transfer
--yo3-revival override: ProjectIgnis' script, which doesn't run a negated effect that is Spell Transfer's own: an
--opponent's Spell Transfer that had become a Spell ran this activation again and again, with the same chain values, until
--the stack overflowed (batch 6's fuzz). The effect is still negated.
local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetCategory(CATEGORY_DISABLE)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_CHAINING)
	e1:SetCondition(s.condition)
	e1:SetTarget(s.target)
	e1:SetOperation(s.activate)
	c:RegisterEffect(e1)
end
function s.condition(e,tp,eg,ep,ev,re,r,rp)
	return re:IsActiveType(TYPE_SPELL) and re:IsHasType(EFFECT_TYPE_ACTIVATE) and Duel.IsChainNegatable(ev) and ep~=tp
end
function s.target(e,tp,eg,ep,ev,re,r,rp,chk)
	if chk==0 then return true end
	Duel.SetOperationInfo(0,CATEGORY_DISABLE,eg,1,0,0)
end
function s.activate(e,tp,eg,ep,ev,re,r,rp)
	Duel.NegateEffect(ev)
	local c=e:GetHandler()
	local tc=re:GetHandler()
	local tpe=tc:GetType()
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_SINGLE)
	e1:SetCode(EFFECT_CHANGE_TYPE)
	e1:SetValue(tpe)
	e1:SetReset(RESET_EVENT+RESETS_STANDARD-RESET_TURN_SET)
	c:RegisterEffect(e1)
	local te=tc:GetActivateEffect()
	if not te or te:GetOperation()==s.activate or tc:IsOriginalCode(id) then return end
	local tg=te:GetTarget()
	local op=te:GetOperation()
	e:SetCategory(te:GetCategory())
	e:SetProperty(te:GetProperty())
	Duel.ClearTargetCard()
	if tg then tg(e,tp,eg,ep,ev,re,r,rp,1) end
	Duel.BreakEffect()
	if op then op(e,tp,eg,ep,ev,re,r,rp) end
	if tc:IsType(TYPE_EQUIP+TYPE_CONTINUOUS) then
		local code=tc:GetOriginalCode()
		c:CopyEffect(code,RESET_EVENT+RESETS_STANDARD-RESET_TURN_SET,1)
		c:CancelToGrave()
	end
	if (tpe&TYPE_FIELD)~=0 then
		local of=Duel.GetFieldCard(1-tp,LOCATION_FZONE,0)
		if of then Duel.Destroy(of,REASON_RULE) end
		of=Duel.GetFieldCard(tp,LOCATION_FZONE,0)
		if of and Duel.Destroy(of,REASON_RULE)==0 then Duel.SendtoGrave(tc,REASON_RULE) end
		Duel.MoveSequence(tc,5)
	end
end