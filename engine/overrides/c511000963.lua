--Dromi the Sacred Shackles
--yo3-revival override: ProjectIgnis' script, remembering the targeted monster without asking whether it is still related to
--the effect when it leaves the field. Upstream checked `oc:IsRelateToEffect(e)` in its EVENT_LEAVE_FIELD condition, but a
--card that has left the field is no longer related to any effect, so the damage never came. It is also only for a monster
--that leaves "because of a card effect": one destroyed by battle ends it with no damage.
local s,id=GetID()
function s.initial_effect(c)
	--Activate
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCode(EVENT_FREE_CHAIN)
	e1:SetProperty(EFFECT_FLAG_CARD_TARGET)
	e1:SetTarget(s.target)
	e1:SetOperation(s.activate)
	c:RegisterEffect(e1)
end
function s.filter(c)
	return c:IsFaceup() and c:IsRace(RACE_DIVINE)
end
function s.target(e,tp,eg,ep,ev,re,r,rp,chk,chkc)
	if chkc then return false end
	if chk==0 then return Duel.IsExistingTarget(Card.IsFaceup,tp,0,LOCATION_MZONE,1,nil)
		and Duel.IsExistingTarget(s.filter,tp,LOCATION_MZONE,0,1,nil) end
	Duel.Hint(HINT_SELECTMSG,tp,HINTMSG_SELF)
	Duel.SelectTarget(tp,s.filter,tp,LOCATION_MZONE,0,1,1,nil)
	Duel.Hint(HINT_SELECTMSG,tp,HINTMSG_OPPO)
	local g=Duel.SelectTarget(tp,Card.IsFaceup,tp,0,LOCATION_MZONE,1,1,nil)
	e:SetLabelObject(g:GetFirst())
end
function s.activate(e,tp,eg,ep,ev,re,r,rp)
	local g=Duel.GetChainInfo(0,CHAININFO_TARGET_CARDS)
	local oc=e:GetLabelObject()
	local sc=g:GetFirst()
	if sc==oc then sc=g:GetNext() end
	if sc and s.filter(sc) and sc:IsRelateToEffect(e) and oc:IsFaceup() and oc:IsControler(1-tp) and oc:IsRelateToEffect(e) then
		local e1=Effect.CreateEffect(e:GetHandler())
		e1:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_CONTINUOUS)
		e1:SetCode(EVENT_LEAVE_FIELD)
		e1:SetLabel(math.abs(sc:GetAttack()-oc:GetAttack()))
		e1:SetLabelObject(oc)
		e1:SetCondition(s.damcon)
		e1:SetOperation(s.damop)
		Duel.RegisterEffect(e1,tp)
	end
end
function s.damcon(e,tp,eg,ep,ev,re,r,rp)
	local oc=e:GetLabelObject()
	if e:GetLabel()<=0 then
		e:Reset()
		return false
	end
	if not eg:IsContains(oc) then return false end
	if not oc:IsReason(REASON_EFFECT) then
		e:Reset()
		return false
	end
	return true
end
function s.damop(e,tp,eg,ep,ev,re,r,rp)
	Duel.Damage(1-tp,e:GetLabel(),REASON_EFFECT)
	e:Reset()
end
