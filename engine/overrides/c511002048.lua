--Call of the Living Dead
--yo3-revival override: ProjectIgnis' script, with the revival's 10% taken on the ATK the monster had when it was destroyed
--Scripted by Edo9300
local s,id=GetID()
function s.initial_effect(c)
	--activate
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_ACTIVATE)
	e1:SetCategory(CATEGORY_SPECIAL_SUMMON)
	e1:SetCode(EVENT_FREE_CHAIN)
	e1:SetTarget(s.tg)
	e1:SetOperation(s.op)
	c:RegisterEffect(e1)
	--draw
	local e2=Effect.CreateEffect(c)
	e2:SetCategory(CATEGORY_SPECIAL_SUMMON)
	e2:SetType(EFFECT_TYPE_FIELD+EFFECT_TYPE_TRIGGER_F)
	e2:SetProperty(EFFECT_FLAG_DELAY)
	e2:SetRange(LOCATION_SZONE)
	e2:SetCode(EVENT_BATTLE_DESTROYED)
	e2:SetTarget(s.target)
	e2:SetOperation(s.operation)
	c:RegisterEffect(e2)
	--remain field
	local e3=Effect.CreateEffect(c)
	e3:SetType(EFFECT_TYPE_SINGLE)
	e3:SetCode(EFFECT_REMAIN_FIELD)
	c:RegisterEffect(e3)
 end
 function s.fil(c,e,tp)
	return c:IsCanBeSpecialSummoned(e,0,tp,false,false)
 end
 function s.tg(e,tp,eg,ep,ev,re,r,rp,chk)
	if chk==0 then return Duel.GetLocationCount(tp,LOCATION_MZONE)>0 
		and Duel.IsExistingMatchingCard(s.fil,tp,LOCATION_GRAVE,0,1,nil,e,tp) end
	Duel.SetOperationInfo(0,CATEGORY_SPECIAL_SUMMON,nil,1,tp,LOCATION_GRAVE)
 end
 function s.op(e,tp,eg,ep,ev,re,r,rp)
	local ft=Duel.GetLocationCount(tp,LOCATION_MZONE)
	if ft<=0 then return end
	if Duel.IsPlayerAffectedByEffect(tp,CARD_BLUEEYES_SPIRIT) then ft=1 end
	Duel.Hint(HINT_SELECTMSG,tp,HINTMSG_SPSUMMON)
	local g=Duel.SelectMatchingCard(tp,s.fil,tp,LOCATION_GRAVE,0,1,ft,nil,e,tp)
	if #g>0 then
		g:ForEach(function(tc)
			Duel.SpecialSummonStep(tc,0,tp,tp,false,false,POS_FACEUP_ATTACK)
			local e1=Effect.CreateEffect(e:GetHandler())
			e1:SetType(EFFECT_TYPE_SINGLE)
			e1:SetCode(EFFECT_CHANGE_RACE)
			e1:SetProperty(EFFECT_FLAG_CANNOT_DISABLE)
			e1:SetValue(RACE_ZOMBIE)
			e1:SetReset(RESET_EVENT+RESETS_STANDARD)
			tc:RegisterEffect(e1)
			local e2=Effect.CreateEffect(e:GetHandler())
			e2:SetType(EFFECT_TYPE_SINGLE)
			e2:SetCode(EFFECT_SET_DEFENSE)
			e2:SetProperty(EFFECT_FLAG_CANNOT_DISABLE)
			e2:SetValue(0)
			e2:SetReset(RESET_EVENT+RESETS_STANDARD)
			tc:RegisterEffect(e2)
		end)
		Duel.SpecialSummonComplete()
	end
 end
 function s.filter(c,e,tp)
	return c:IsLocation(LOCATION_GRAVE) and c:GetPreviousControler()==tp and c:IsReason(REASON_BATTLE) and c:GetOriginalRace()~=RACE_ZOMBIE
		and c:IsCanBeSpecialSummoned(e,0,tp,false,false)
 end
 function s.target(e,tp,eg,ep,ev,re,r,rp,chk,chkc)
	if chk==0 then return Duel.GetLocationCount(tp,LOCATION_MZONE)>0 and eg:IsExists(s.filter,1,nil,e,tp) end
	eg:Filter(s.filter,nil,e,tp):GetFirst():CreateEffectRelation(e)
	Duel.SetOperationInfo(0,CATEGORY_SPECIAL_SUMMON,eg:Filter(s.filter,nil,e,tp),1,0,0)
 end
 function s.operation(e,tp,eg,ep,ev,re,r,rp)
	local tc=eg:Filter(s.filter,nil,e,tp):GetFirst()
	if tc and tc:IsRelateToEffect(e) then
		Duel.SpecialSummonStep(tc,0,tp,tp,false,false,POS_FACEUP_ATTACK)
		local e1=Effect.CreateEffect(e:GetHandler())
		e1:SetType(EFFECT_TYPE_SINGLE)
		e1:SetCode(EFFECT_CHANGE_RACE)
		e1:SetProperty(EFFECT_FLAG_CANNOT_DISABLE)
		e1:SetValue(RACE_ZOMBIE)
		e1:SetReset(RESET_EVENT+RESETS_STANDARD)
		tc:RegisterEffect(e1)
		local e2=Effect.CreateEffect(e:GetHandler())
		e2:SetType(EFFECT_TYPE_SINGLE)
		e2:SetCode(EFFECT_SET_DEFENSE)
		e2:SetProperty(EFFECT_FLAG_CANNOT_DISABLE)
		e2:SetValue(0)
		e2:SetReset(RESET_EVENT+RESETS_STANDARD)
		tc:RegisterEffect(e2)
		--yo3-revival: the 10% grows the monster's own ATK: its printed ATK, or the one an earlier revival by this effect gave
		--it (the mark below, kept into the Graveyard), never a boost of another card's (an Equip Spell), which falls off
		--with that card; it is set as the monster's own ATK, so a boost given afterwards applies on top. Upstream read its
		--ATK in the Graveyard and never reset the effect.
		local own=tc:GetTextAttack()
		local mark=tc:GetFlagEffect(id)>0 and tc:GetFlagEffectLabel(id) or 0
		if mark>0 and mark%100000==Duel.GetTurnCount() then own=mark//100000 end
		local atk=own+own//10
		local e3=Effect.CreateEffect(e:GetHandler())
		e3:SetType(EFFECT_TYPE_SINGLE)
		e3:SetProperty(EFFECT_FLAG_CANNOT_DISABLE)
		e3:SetCode(EFFECT_SET_BASE_ATTACK)
		e3:SetValue(atk)
		e3:SetReset(RESET_EVENT+RESETS_STANDARD)
		tc:RegisterEffect(e3)
		--The mark, written as the monster leaves the field by battle (its own ATK then, the Equip Spells' boosts not
		--counted, and the turn): kept into the Graveyard and read by the revival that follows at once, on that turn only;
		--any other way off the field clears it. (The core's reset for arriving on the field reaches a mark written in the
		--destruction itself, so the turn, not that reset, keeps an old mark from counting.)
		local e4=Effect.CreateEffect(e:GetHandler())
		e4:SetType(EFFECT_TYPE_SINGLE+EFFECT_TYPE_CONTINUOUS)
		e4:SetCode(EVENT_LEAVE_FIELD_P)
		e4:SetProperty(EFFECT_FLAG_CANNOT_DISABLE+EFFECT_FLAG_UNCOPYABLE)
		e4:SetOperation(function(ef)
			local h=ef:GetHandler()
			h:ResetFlagEffect(id)
			if h:IsReason(REASON_BATTLE) then
				h:RegisterFlagEffect(id,RESET_EVENT|(RESETS_STANDARD_EXC_GRAVE&~RESET_TOFIELD),0,1,h:GetBaseAttack()*100000+Duel.GetTurnCount())
			end
		end)
		e4:SetReset(RESET_EVENT|(RESETS_STANDARD&~RESET_TOFIELD))
		tc:RegisterEffect(e4)
		Duel.SpecialSummonComplete()
	end
 end