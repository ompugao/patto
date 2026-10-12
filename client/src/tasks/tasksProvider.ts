import * as vscode from 'vscode';
import { taskLabel, type TaskInformation, type TaskLocation } from './taskLabel';

export class TasksProvider implements vscode.TreeDataProvider<TaskItem> {
	private tasks: TaskItem[] = [];

	private readonly _onDidChangeTreeData = new vscode.EventEmitter<TaskItem | undefined | null | void>();
	readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

	getTreeItem(element: TaskItem): vscode.TreeItem {
		return element;
	}

	getChildren(element?: TaskItem): Thenable<TaskItem[]> {
		return Promise.resolve(element ? [] : this.tasks);
	}

	refresh(tasks: TaskInformation[]): void {
		this.tasks = tasks.map(task => new TaskItem(taskLabel(task), task.location));
		this._onDidChangeTreeData.fire();
	}
}

class TaskItem extends vscode.TreeItem {
	constructor(label: string, location: TaskLocation | undefined) {
		super(label, vscode.TreeItemCollapsibleState.None);
		this.tooltip = label;

		if (location && location.uri) {
			this.command = {
				command: 'vscode.open',
				title: 'Open Task',
				arguments: [
					vscode.Uri.parse(location.uri),
					{
						selection: new vscode.Range(
							location.range.start.line,
							location.range.start.character,
							location.range.end.line,
							location.range.end.character
						)
					}
				]
			};
		}
	}
}
