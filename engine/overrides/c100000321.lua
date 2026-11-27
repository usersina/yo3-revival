--yo3-revival override: ProjectIgnis' script used EVENT_BATTLE_END, a timing the core stopped raising with Master Rule 3, so the
--card could never do anything (batch 30's fuzz). It also negated the flip monster only after the battle, when its Flip effect had
--already been used: this negates the face-down Defense Position monster when the attack on it is declared, as the text says
--("Negate the effects of face-down Defense Position monsters attacked by this card"), whatever its type.
--酔いどれタイガー
local s,id=GetID()
function s.initial_effect(c)
	--Disable
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_SINGLE+EFFECT_TYPE_CONTINUOUS)
	e1:SetCode(EVENT_ATTACK_ANNOUNCE)
	e1:SetOperation(s.disop)
	c:RegisterEffect(e1)
end
function s.disop(e,tp,eg,ep,ev,re,r,rp)
	local c=e:GetHandler()
	local tc=Duel.GetAttackTarget()
	if tc and tc:IsFacedown() and tc:IsDefensePos() and Duel.GetAttacker()==c then
		local e1=Effect.CreateEffect(c)
		e1:SetType(EFFECT_TYPE_SINGLE)
		e1:SetCode(EFFECT_DISABLE)
		e1:SetReset(RESET_EVENT+RESETS_STANDARD_EXC_GRAVE)
		tc:RegisterEffect(e1)
		local e2=Effect.CreateEffect(c)
		e2:SetType(EFFECT_TYPE_SINGLE)
		e2:SetCode(EFFECT_DISABLE_EFFECT)
		e2:SetReset(RESET_EVENT+RESETS_STANDARD_EXC_GRAVE)
		tc:RegisterEffect(e2)
	end
end
