import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:test/test.dart';

import '../lib/codex_client.dart';

void main() {
  group('CodexClient', () {
    test('embed', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/v1/embeddings');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['texts'], ['hi']);
        return http.Response(
          jsonEncode({
            'embeddings': [
              [1, 2, 3],
            ],
          }),
          200,
        );
      });
      final client = CodexClient('http://x', httpClient: mock);
      final emb = await client.embed(['hi']);
      expect(emb.length, 1);
      expect(emb.first.length, 3);
    });

    test('upsert', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/v1/vector/upsert');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect((body['vectors'] as List).length, 1);
        return http.Response(jsonEncode({'inserted': 1}), 200);
      });
      final client = CodexClient('http://x', httpClient: mock);
      final count = await client.upsert([
        VectorRecord(id: 1, values: [1, 2], document: 'd'),
      ]);
      expect(count, 1);
    });

    test('query', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/v1/vector/query');
        return http.Response(
          jsonEncode({
            'results': [
              {'id': 1, 'document': 'doc'},
            ],
          }),
          200,
        );
      });
      final client = CodexClient('http://x', httpClient: mock);
      final res = await client.query([1, 2], 1);
      expect(res.length, 1);
      expect(res.first.document, 'doc');
    });

    test('chat', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/v1/chat');
        return http.Response(jsonEncode({'reply': 'ok'}), 200);
      });
      final client = CodexClient('http://x', httpClient: mock);
      final reply = await client.chat('', [
        Message(role: 'user', content: 'hi'),
      ]);
      expect(reply, 'ok');
    });

    test('ragAnswer', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/v1/rag/answer');
        return http.Response(
          jsonEncode({
            'answer': '42',
            'contexts': ['doc'],
          }),
          200,
        );
      });
      final client = CodexClient('http://x', httpClient: mock);
      final reply = await client.ragAnswer('?', 1, '', false);
      expect(reply.answer, '42');
      expect(reply.references.length, 1);
    });

    test('mul', () async {
      final mock = MockClient((request) async {
        expect(request.url.path, '/v1/mul');
        final body = jsonDecode(request.body) as Map<String, dynamic>;
        expect(body['source'], '1');
        expect(body['from'], 'mul');
        expect(body['to'], 'rust');
        return http.Response(jsonEncode({'output': 'translated'}), 200);
      });
      final client = CodexClient('http://x', httpClient: mock);
      final out = await client.mul('1', 'mul', 'rust');
      expect(out, 'translated');
    });
  });
}
