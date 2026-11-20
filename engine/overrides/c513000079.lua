--ダーク・ダイブ・ボンバー (Anime)
--Dark Strike Fighter (Anime)
--yo3-revival: our own script for the anime card, a RECONSTRUCTION (ProjectIgnis has only a placeholder, no script). The
--rules are rebuilt from the community's description of the duels (5D's episodes 21 and 134), not verified against the
--footage; the choices made are written in the card's text (engine/anime/records.json) and docs/anime-cards.md:
--  * a Level 7 Synchro Monster (the printed Synchro procedure, reused);
--  * an ignition effect, so it can be used in either Main Phase;
--  * "once per turn" is for each copy, not by name (SetCountLimit(1) belongs to this effect object);
--  * the cost is Tributing ANOTHER monster you control (this card is excluded from the check and from the choice);
--  * the damage is the Tributed monster's Level at the moment it is Tributed, x 200, kept with this activation: it is
--    dealt even if this card has left the field before the effect resolves;
--  * "this card cannot attack during the turn you activate this effect": the effect cannot be activated if this card
--    already declared an attack this turn, and activating it forbids this card to attack for the rest of the turn. The
--    restriction is put on at activation (in the cost), so negating the effect does not lift it.
local s,id=GetID()
function s.initial_effect(c)
	--synchro summon
	Synchro.AddProcedure(c,nil,1,1,Synchro.NonTuner(nil),1,99)
	c:EnableReviveLimit()
	--damage
	local e1=Effect.CreateEffect(c)
	e1:SetDescription(aux.Stringid(id,0))
	e1:SetCategory(CATEGORY_DAMAGE)
	e1:SetType(EFFECT_TYPE_IGNITION)
	e1:SetProperty(EFFECT_FLAG_PLAYER_TARGET)
	e1:SetRange(LOCATION_MZONE)
	e1:SetCountLimit(1)
	e1:SetCondition(s.condition)
	e1:SetCost(s.cost)
	e1:SetTarget(s.target)
	e1:SetOperation(s.operation)
	c:RegisterEffect(e1)
end
function s.condition(e,tp,eg,ep,ev,re,r,rp)
	return e:GetHandler():GetAttackAnnouncedCount()==0
end
function s.cost(e,tp,eg,ep,ev,re,r,rp,chk)
	local c=e:GetHandler()
	if chk==0 then return Duel.CheckReleaseGroupCost(tp,nil,1,1,false,nil,c) end
	local g=Duel.SelectReleaseGroupCost(tp,nil,1,1,false,nil,c)
	--the Level it has now, before it leaves the field
	e:SetLabel(math.max(g:GetFirst():GetLevel(),0)*200)
	Duel.Release(g,REASON_COST)
	--cannot attack this turn: installed here, at activation
	local e1=Effect.CreateEffect(c)
	e1:SetType(EFFECT_TYPE_SINGLE)
	e1:SetCode(EFFECT_CANNOT_ATTACK_ANNOUNCE)
	e1:SetProperty(EFFECT_FLAG_OATH+EFFECT_FLAG_CANNOT_DISABLE)
	e1:SetReset(RESET_EVENT|RESETS_STANDARD|RESET_PHASE|PHASE_END)
	c:RegisterEffect(e1)
end
function s.target(e,tp,eg,ep,ev,re,r,rp,chk)
	if chk==0 then return true end
	Duel.SetTargetPlayer(1-tp)
	Duel.SetTargetParam(e:GetLabel())
	Duel.SetOperationInfo(0,CATEGORY_DAMAGE,nil,0,1-tp,e:GetLabel())
	e:SetLabel(0)
end
function s.operation(e,tp,eg,ep,ev,re,r,rp)
	--the amount belongs to this activation, whatever became of this card
	local p,d=Duel.GetChainInfo(0,CHAININFO_TARGET_PLAYER,CHAININFO_TARGET_PARAM)
	Duel.Damage(p,d,REASON_EFFECT)
end
