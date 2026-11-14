--強化支援メカ・ヘビーウェポン
--Heavy Mech Support Platform
local s,id=GetID()
function s.initial_effect(c)
	--The old Union rules, as the client's text has them: Special Summoned in face-up Attack Position and 1 Union per
	--monster; "If the equipped monster would be destroyed, destroy this card instead", by anything (yo3-revival
	--override: today's text takes either position)
	aux.AddUnionProcedure(c,aux.FilterBoolFunction(Card.IsRace,RACE_MACHINE),true,false)
	--Atk up
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_EQUIP)
	e1:SetCode(EFFECT_UPDATE_ATTACK)
	e1:SetValue(500)
	c:RegisterEffect(e1)
	--Def up
	local e2=e1:Clone()
	e2:SetCode(EFFECT_UPDATE_DEFENSE)
	c:RegisterEffect(e2)
end
