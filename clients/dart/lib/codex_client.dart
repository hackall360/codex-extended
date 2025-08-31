import 'dart:convert';

import 'package:http/http.dart' as http;

class Message {
  final String role;
  final String content;

  Message({required this.role, required this.content});

  factory Message.fromJson(Map<String, dynamic> json) {
    return Message(
      role: json['role'] as String,
      content: json['content'] as String,
    );
  }

  Map<String, dynamic> toJson() => {'role': role, 'content': content};
}

class Result {
  final int? id;
  final String document;

  Result({this.id, required this.document});

  factory Result.fromJson(Map<String, dynamic> json) {
    return Result(id: json['id'] as int?, document: json['document'] as String);
  }

  Map<String, dynamic> toJson() => {
    if (id != null) 'id': id,
    'document': document,
  };
}

class RagReply {
  final String answer;
  final List<Result> references;

  RagReply({required this.answer, required this.references});

  factory RagReply.fromJson(Map<String, dynamic> json) {
    final contexts = (json['contexts'] as List? ?? []);
    final refs = contexts
        .map((doc) => Result(document: doc as String))
        .toList();
    return RagReply(answer: json['answer'] as String, references: refs);
  }

  Map<String, dynamic> toJson() => {
    'answer': answer,
    'contexts': references.map((r) => r.document).toList(),
  };
}

class VectorRecord {
  final int id;
  final List<double> values;
  final String document;

  VectorRecord({
    required this.id,
    required this.values,
    required this.document,
  });

  factory VectorRecord.fromJson(Map<String, dynamic> json) {
    return VectorRecord(
      id: json['id'] as int,
      values: (json['values'] as List)
          .map((e) => (e as num).toDouble())
          .toList(),
      document: json['document'] as String,
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'values': values,
    'document': document,
  };
}

class CodexClient {
  final String baseUrl;
  final http.Client _client;

  CodexClient(String baseUrl, {http.Client? httpClient})
    : baseUrl = baseUrl.replaceFirst(RegExp(r'/+$'), ''),
      _client = httpClient ?? http.Client();

  Future<T> _post<T>(
    String path,
    Object body,
    T Function(dynamic json) parse,
  ) async {
    final uri = Uri.parse('$baseUrl$path');
    final resp = await _client.post(
      uri,
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(body),
    );
    if (resp.statusCode < 200 || resp.statusCode >= 300) {
      throw http.ClientException('HTTP ${resp.statusCode}: ${resp.body}', uri);
    }
    final data = jsonDecode(resp.body);
    return parse(data);
  }

  Future<List<List<double>>> embed(List<String> texts) async {
    return _post('/v1/embeddings', {'texts': texts}, (json) {
      final list = json['embeddings'] as List;
      return list
          .map<List<double>>(
            (e) =>
                (e as List).map<double>((n) => (n as num).toDouble()).toList(),
          )
          .toList();
    });
  }

  Future<int> upsert(List<VectorRecord> vectors) {
    final payload = {'vectors': vectors.map((v) => v.toJson()).toList()};
    return _post(
      '/v1/vector/upsert',
      payload,
      (json) => json['inserted'] as int,
    );
  }

  Future<List<Result>> query(List<double> vector, int topK) {
    final payload = {'vector': vector, 'top_k': topK};
    return _post('/v1/vector/query', payload, (json) {
      final list = json['results'] as List;
      return list
          .map<Result>((e) => Result.fromJson(e as Map<String, dynamic>))
          .toList();
    });
  }

  Future<String> chat(String tier, List<Message> messages) {
    final payload = {
      if (tier.isNotEmpty) 'tier': tier,
      'messages': messages.map((m) => m.toJson()).toList(),
    };
    return _post('/v1/chat', payload, (json) => json['reply'] as String);
  }

  Future<RagReply> ragAnswer(
    String question,
    int topK,
    String tier,
    bool translate,
  ) {
    final payload = {
      'question': question,
      'top_k': topK,
      if (tier.isNotEmpty) 'tier': tier,
      if (translate) 'translate': translate,
    };
    return _post(
      '/v1/rag/answer',
      payload,
      (json) => RagReply.fromJson(json as Map<String, dynamic>),
    );
  }
}
