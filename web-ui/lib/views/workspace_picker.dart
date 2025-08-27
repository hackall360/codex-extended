import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';

class WorkspacePickerView extends StatefulWidget {
  const WorkspacePickerView({super.key});

  @override
  State<WorkspacePickerView> createState() => _WorkspacePickerViewState();
}

class _WorkspacePickerViewState extends State<WorkspacePickerView> {
  String? _path;

  Future<void> _pick() async {
    final result = await FilePicker.platform.getDirectoryPath();
    if (result != null) {
      setState(() => _path = result);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Text(_path ?? 'No directory selected'),
        ElevatedButton(
          onPressed: _pick,
          child: const Text('Select Directory'),
        ),
      ],
    );
  }
}
