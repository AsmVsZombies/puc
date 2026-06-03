#pragma once

// Shared RunParams JSON helpers for the per-calculator shim TUs. Header-only and
// calculator-agnostic (no seml headers) so it can be included from every
// calc_*.cpp without dragging another calculator's headers into the TU.

#include <stdexcept>
#include <string>
#include <vector>

#include "rapidjson/document.h"

namespace puc_params {

// Parses the params JSON (empty/null => "{}"). Throws on malformed JSON.
inline rapidjson::Document parse(const char* params_json)
{
    rapidjson::Document d;
    const char* pj = (params_json && *params_json) ? params_json : "{}";
    d.Parse(pj);
    if (d.HasParseError())
        throw std::runtime_error("invalid JSON in params");
    return d;
}

inline int opt_int(const rapidjson::Value& v, const char* key, int fallback)
{
    if (!v.IsObject())
        return fallback;
    auto it = v.FindMember(key);
    if (it == v.MemberEnd() || it->value.IsNull())
        return fallback;
    if (!it->value.IsInt())
        throw std::runtime_error(std::string("param '") + key + "' must be an integer");
    return it->value.GetInt();
}

inline bool opt_bool(const rapidjson::Value& v, const char* key, bool fallback)
{
    if (!v.IsObject())
        return fallback;
    auto it = v.FindMember(key);
    if (it == v.MemberEnd() || it->value.IsNull())
        return fallback;
    if (!it->value.IsBool())
        throw std::runtime_error(std::string("param '") + key + "' must be a boolean");
    return it->value.GetBool();
}

inline unsigned opt_uint(const rapidjson::Value& v, const char* key, unsigned fallback)
{
    int n = opt_int(v, key, static_cast<int>(fallback));
    if (n < 0)
        throw std::runtime_error(std::string("param '") + key + "' must be non-negative");
    return static_cast<unsigned>(n);
}

// Optional array-of-int param. Missing/null => empty vector.
inline std::vector<int> opt_int_array(const rapidjson::Value& v, const char* key)
{
    std::vector<int> out;
    if (!v.IsObject())
        return out;
    auto it = v.FindMember(key);
    if (it == v.MemberEnd() || it->value.IsNull())
        return out;
    if (!it->value.IsArray())
        throw std::runtime_error(std::string("param '") + key + "' must be an array");
    for (const auto& e : it->value.GetArray()) {
        if (!e.IsInt())
            throw std::runtime_error(std::string("param '") + key + "' entries must be integers");
        out.push_back(e.GetInt());
    }
    return out;
}

} // namespace puc_params
