#include "Epochlib.hpp"
#include "RedisConnector.hpp"

RedisConnector::RedisConnector(EpochlibConfigRedis _config) {
	this->config = _config;
	this->context = NULL;

	this->_reconnect(false);
}
RedisConnector::~RedisConnector() {
	if (this->context != NULL) {
		redisFree(this->context);
	}
}

EpochlibRedisExecute RedisConnector::execute(const char *format, ...) {
	this->_reconnect(0);

	va_list ap;
	EpochlibRedisExecute returnObj;
	redisReply *reply = NULL;

	while (reply == NULL) {
		// Lock, execute, unlock
		this->contextMutex.lock();
		va_start(ap, format);
		reply = (redisReply *)redisvCommand(this->context, format, ap);
		va_end(ap);
		this->contextMutex.unlock();

		if (reply->type == REDIS_REPLY_ERROR) {
			returnObj.success = false;
			returnObj.message = reply->str;
			this->config.logger->log("[Redis] Error command " + std::string(reply->str));
		}
		else {
			returnObj.success = true;

			if (reply->type == REDIS_REPLY_STRING) {
				returnObj.message = reply->str;
			}
			else if (reply->type == REDIS_REPLY_INTEGER) {
				std::stringstream IntToString;
				IntToString << reply->integer;
				returnObj.message = IntToString.str();
			}
		}
	}

	freeReplyObject(reply);

	return returnObj;
}

void RedisConnector::_reconnect(bool _force) {
	// Security context lock
	this->contextMutex.lock();

	if (this->context == NULL || _force) {
		int retries = 0;
		struct timeval timeout { 1, 50000 };

		do {
			if (this->context != NULL) {
				redisFree(this->context);
			}

			this->context = redisConnectWithTimeout(this->config.ip.c_str(), this->config.port, timeout);

			if (this->context->err) {
				this->config.logger->log("[Redis] " + std::string(this->context->errstr));
			}

			retries++;
		} while (this->context == NULL || (this->context->err & (REDIS_ERR_IO || REDIS_ERR_EOF) && retries < REDISCONNECTOR_MAXCONNECTION_RETRIES));

		/* Too many retries -> exit server with log */
		if (retries == REDISCONNECTOR_MAXCONNECTION_RETRIES) {
			this->config.logger->log("[Redis] Server not reachable");
			exit(1);
		}

		/* Password given -> AUTH */
		if (!this->config.password.empty()) {
			redisReply *authReply = NULL;

			while (authReply == NULL) {
				authReply = (redisReply *)redisCommand(this->context, "AUTH %s", this->config.password.c_str());
			}
			if (authReply->type == REDIS_REPLY_STRING) {
				if (strcmp(authReply->str, "OK") == 0) {
					this->config.logger->log("[Redis] Could not authenticate: " + std::string(authReply->str));
				}
			}

			freeReplyObject(authReply);
		}

		/* Database index given -> change database */
		if (this->config.dbIndex > 0) {
			redisReply *selectReply = NULL;

			while (selectReply == NULL) {
				selectReply = (redisReply *)redisCommand(this->context, "SELECT %d", this->config.dbIndex);
			}
			if (selectReply->type == REDIS_REPLY_STRING) {
				if (strcmp(selectReply->str, "OK") == 0) {
					this->config.logger->log("[Redis] Could not change database: " + std::string(selectReply->str));
				}
			}

			freeReplyObject(selectReply);
		}
	}

	// Unlock
	this->contextMutex.unlock();
}
