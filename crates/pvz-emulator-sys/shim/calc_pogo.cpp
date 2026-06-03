// pogo calculator dispatch — see calc_pos.cpp for the per-TU rationale.

#include "rapidjson/document.h"

#include "seml/pogo/run.h"

#include "params.h"

std::string puc_dispatch_pogo(const char* scenario_json, const char* params_json)
{
    Config config = read_json_string(scenario_json);
    auto params = puc_params::parse(params_json);

    pogo::RunParams p;
    p.repeat = puc_params::opt_int(params, "repeat", p.repeat);
    p.thread_num = puc_params::opt_uint(params, "threadNum", p.thread_num);

    return pogo::result_to_json(config, p, pogo::simulate(config, p));
}
