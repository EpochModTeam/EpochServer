#include <string>
#include "../../deps/happyhttp/happyhttp.h"
#include "../../deps/rapidjson/include/rapidjson/document.h"

#ifndef __STEAMAPI_H__
#define __STEAMAPI_H__

struct SteamAPIResponseContent {
	short int Status;
	size_t ByteCount;
	std::string Content;
};
typedef std::map<std::string, std::string> SteamAPIQuery;

class SteamAPI {
public:
	std::string _apiKey;
	SteamAPIResponseContent *_responseContent;

	bool _sendRequest(std::string URI, SteamAPIQuery Query);

public:
	SteamAPI(std::string APIKey);
	~SteamAPI();

	bool GetPlayerBans(std::string SteamIds, rapidjson::Document *Document);
	bool GetPlayerSummaries(std::string SteamIds, rapidjson::Document *Document);
};

#endif