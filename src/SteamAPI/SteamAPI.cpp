#include "SteamAPI.hpp"
#include <sstream>
#include <fstream>

void SteamAPI_CB_httpBegin(const happyhttp::Response *_response, void *_userdata) {
	SteamAPI *_this = (SteamAPI*)(_userdata);

	_this->_responseContent = new SteamAPIResponseContent;
	_this->_responseContent->ByteCount = 0;
	_this->_responseContent->Status = _response->getstatus();
}
void SteamAPI_CB_httpGetData(const happyhttp::Response *_response, void *_userdata, const unsigned char *_data, int _n) {
	SteamAPI *_this = (SteamAPI*)(_userdata);

	_this->_responseContent->Content.append((const char*)_data, _n);
}
void SteamAPI_CB_httpComplete(const happyhttp::Response *_response, void *_userdata) { }

SteamAPI::SteamAPI(std::string _apiKey) {
	this->_apiKey = _apiKey;
	this->_responseContent = NULL;
}
SteamAPI::~SteamAPI() {
	if (this->_responseContent != NULL) {
		delete this->_responseContent;
	}
}

bool SteamAPI::GetPlayerBans(std::string _steamIds, rapidjson::Document *_document) {
	SteamAPIQuery Query;
	Query.insert(std::pair<std::string, std::string>("steamids", _steamIds));

	if (this->_sendRequest("/ISteamUser/GetPlayerBans/v1/", Query)) {
		_document->Parse(this->_responseContent->Content.c_str());
		return true;
	}
	else {
		return false;
	}
}

bool SteamAPI::GetPlayerSummaries(std::string _steamIds, rapidjson::Document *_document) {
	SteamAPIQuery Query;
	Query.insert(std::pair<std::string, std::string>("steamids", _steamIds));

	if (this->_sendRequest("/ISteamUser/GetPlayerSummaries/v0002/", Query)) {
		_document->Parse(this->_responseContent->Content.c_str());
		return true;
	}
	else {
		return false;
	}
}

bool SteamAPI::_sendRequest(std::string _uri, SteamAPIQuery _query) {

	// Add API key
	_query.insert(_query.begin(), std::pair<std::string, std::string>("key", this->_apiKey));

	// Build query
	std::stringstream queryStream;
	for (SteamAPIQuery::iterator it = _query.begin(); it != _query.end(); ++it) {
		if (queryStream.rdbuf()->in_avail() > 0) {
			queryStream << "&";
		}
		queryStream << it->first << "=" << it->second;
	}
	std::string query = _uri + "?" + queryStream.str();

	
	// Setup HTTP request
	happyhttp::Connection connection("api.steampowered.com", 80);
	connection.setcallbacks(SteamAPI_CB_httpBegin, SteamAPI_CB_httpGetData, SteamAPI_CB_httpComplete, this);
	if (connection.request("GET", query.c_str())) {
		return false;
	}

	while (connection.outstanding()) {
		if (connection.pump()) {
			return false;
		}
	}

	return this->_responseContent->Status == happyhttp::OK ? true : false;
}
