// The core's queries, made safe to ask between two steps of a duel. Compiled with the core by build.rs; core.rs calls
// these instead of OCG_DuelQuery, OCG_DuelQueryLocation and OCG_DuelQueryField.
//
// A step can queue the arguments of its next Lua call and stop there, because it also wrote a message: an extra
// Normal Summon hints the card that grants it, then queues that card's operation with the summoned monster as its
// argument (operations.cpp, the summon's last step). A query asked at that point can call Lua too (a monster's ATK
// from a value function, called with the effect and the card), and that call took the queued argument along with its
// own: "incorrect parameter count", and the operation later ran without its card (issue #9, Brain Research Lab).
// The server asks after every step (the field report, MSG_MOVE's reasons, the bot's look-ahead), so each query runs
// with the queue set aside and puts it back afterwards.
#include <utility>
#include "ocgapi.h"
#include "duel.h"
#include "interpreter.h"

namespace {
template<typename Query>
void* aside(OCG_Duel ocg_duel, Query query) {
	auto& params = static_cast<duel*>(ocg_duel)->lua->params;
	auto queued = std::move(params);
	params.clear();
	void* result = query();
	params = std::move(queued);
	return result;
}
}

extern "C" void* YO3_DuelQuery(OCG_Duel ocg_duel, uint32_t* length, const OCG_QueryInfo* info) {
	return aside(ocg_duel, [&] { return OCG_DuelQuery(ocg_duel, length, info); });
}

extern "C" void* YO3_DuelQueryLocation(OCG_Duel ocg_duel, uint32_t* length, const OCG_QueryInfo* info) {
	return aside(ocg_duel, [&] { return OCG_DuelQueryLocation(ocg_duel, length, info); });
}

extern "C" void* YO3_DuelQueryField(OCG_Duel ocg_duel, uint32_t* length) {
	return aside(ocg_duel, [&] { return OCG_DuelQueryField(ocg_duel, length); });
}
