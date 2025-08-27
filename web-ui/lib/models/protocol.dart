import 'package:flutter/foundation.dart';

@immutable
abstract class ProtocolMessage {
  const ProtocolMessage();

  factory ProtocolMessage.fromJson(Map<String, dynamic> json) {
    switch (json['type']) {
      case 'session_opened':
        return SessionOpened(json['id'] as String);
      case 'session_closed':
        return SessionClosed(json['id'] as String);
      case 'event':
        return AgentEvent(json['session'] as String, json['data'] as String);
      case 'config':
        return ConfigForm(Map<String, dynamic>.from(json['fields'] as Map));
      case 'session_exported':
        return SessionExported(
          json['id'] as String,
          List<String>.from(json['history'] as List),
        );
      default:
        return UnknownMessage(json);
    }
  }
}

class SessionOpened extends ProtocolMessage {
  final String id;
  const SessionOpened(this.id);
}

class SessionClosed extends ProtocolMessage {
  final String id;
  const SessionClosed(this.id);
}

class AgentEvent extends ProtocolMessage {
  final String session;
  final String data;
  const AgentEvent(this.session, this.data);
}

class SessionExported extends ProtocolMessage {
  final String id;
  final List<String> history;
  const SessionExported(this.id, this.history);
}

class ConfigForm extends ProtocolMessage {
  final Map<String, dynamic> fields;
  const ConfigForm(this.fields);
}

class UnknownMessage extends ProtocolMessage {
  final Map<String, dynamic> json;
  const UnknownMessage(this.json);
}
