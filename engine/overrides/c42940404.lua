--マシンナーズ・ギアフレーム
--Machina Gearframe
local s,id=GetID()
function s.initial_effect(c)
	--The old Union rules, as the client's text has them: Special Summoned in face-up Attack Position and 1 Union per
	--monster; "If the equipped monster would be destroyed, destroy this card instead", by anything (yo3-revival
	--override: today's text takes either position)
	aux.AddUnionProcedure(c,aux.FilterBoolFunction(Card.IsRace,RACE_MACHINE),true,false)
	--search
	local e1=Effect.CreateEffect(c)
	e1:SetDescription(aux.Stringid(id,2))
	e1:SetCategory(CATEGORY_TOHAND+CATEGORY_SEARCH)
	e1:SetType(EFFECT_TYPE_TRIGGER_O+EFFECT_TYPE_SINGLE)
	e1:SetCode(EVENT_SUMMON_SUCCESS)
	e1:SetTarget(s.stg)
	e1:SetOperation(s.sop)
	c:RegisterEffect(e1)
end
s.listed_series={SET_MACHINA}
s.listed_names={id}
function s.sfilter(c)
	return c:IsSetCard(SET_MACHINA) and c:GetCode()~=id and c:IsMonster() and c:IsAbleToHand()
end
function s.stg(e,tp,eg,ep,ev,re,r,rp,chk)
	if chk==0 then return Duel.IsExistingMatchingCard(s.sfilter,tp,LOCATION_DECK,0,1,nil) end
	Duel.SetOperationInfo(0,CATEGORY_TOHAND,nil,1,tp,LOCATION_DECK)
end
function s.sop(e,tp,eg,ep,ev,re,r,rp)
	Duel.Hint(HINT_SELECTMSG,tp,HINTMSG_ATOHAND)
	local g=Duel.SelectMatchingCard(tp,s.sfilter,tp,LOCATION_DECK,0,1,1,nil)
	if #g>0 then
		Duel.SendtoHand(g,nil,REASON_EFFECT)
		Duel.ConfirmCards(1-tp,g)
	end
end
