--Y－ドラゴン・ヘッド
--Y-Dragon Head
local s,id=GetID()
function s.initial_effect(c)
	--The old Union rules, as the client's text has them: Special Summoned in face-up Attack Position, 1 Union per
	--monster, and "If the equipped monster would be destroyed by battle, destroy this card instead" (yo3-revival
	--override: today's text takes either position and protects from card effects too)
	aux.AddUnionProcedure(c,aux.FilterBoolFunction(Card.IsCode,62651957),true)
	--Atk up
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_EQUIP)
	e1:SetCode(EFFECT_UPDATE_ATTACK)
	e1:SetValue(400)
	c:RegisterEffect(e1)
	--Def up
	local e2=e1:Clone()
	e2:SetCode(EFFECT_UPDATE_DEFENSE)
	c:RegisterEffect(e2)
end
s.listed_names={62651957}
