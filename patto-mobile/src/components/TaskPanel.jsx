// TaskPanel component - displays tasks grouped by deadline

import { useStore, View } from '../lib/store';
import { useEffect } from 'react';
import './TaskPanel.css';

export function TaskPanel() {
    const {
        tasks,
        isLoadingTasks,
        loadTasks,
        setView,
        openNote,
    } = useStore();

    useEffect(() => {
        loadTasks();
    }, [loadTasks]);

    const handleTaskClick = (task) => {
        openNote(task.filePath);
    };

    const TaskSection = ({ title, items, color }) => {
        if (!items || items.length === 0) return null;

        return (
            <section className="task-section">
                <h2 className={`section-title ${color}`}>
                    {title} <span className="count">({items.length})</span>
                </h2>
                <ul className="task-list">
                    {items.map((task, index) => (
                        <li
                            key={`${task.filePath}-${task.lineNumber}-${index}`}
                            className="task-item"
                            onClick={() => handleTaskClick(task)}
                        >
                            <span className="task-content">{task.content}</span>
                            <span className="task-meta">
                                <span className="task-file">{task.fileName}</span>
                                {task.dueDate && (
                                    <span className="task-due">{task.dueDate}</span>
                                )}
                            </span>
                        </li>
                    ))}
                </ul>
            </section>
        );
    };

    return (
        <div className="task-panel">
            <header className="task-header">
                <button className="back-btn" onClick={() => setView(View.FILE_LIST)}>
                    ← Back
                </button>
                <h1>Tasks</h1>
                <button className="refresh-btn" onClick={loadTasks} disabled={isLoadingTasks}>
                    <span className={isLoadingTasks ? 'spinning' : ''}>↻</span>
                </button>
            </header>

            {isLoadingTasks ? (
                <div className="loading">Loading tasks...</div>
            ) : tasks ? (
                <div className="task-sections">
                    <TaskSection title="Overdue" items={tasks.overdue} color="overdue" />
                    <TaskSection title="Today" items={tasks.today} color="today" />
                    <TaskSection title="This Week" items={tasks.thisWeek} color="week" />
                    <TaskSection title="Later" items={tasks.later} color="later" />
                    <TaskSection title="No Deadline" items={tasks.noDeadline} color="none" />

                    {tasks.overdue?.length === 0 &&
                        tasks.today?.length === 0 &&
                        tasks.thisWeek?.length === 0 &&
                        tasks.later?.length === 0 &&
                        tasks.noDeadline?.length === 0 && (
                            <div className="empty-state">No tasks found 🎉</div>
                        )}
                </div>
            ) : (
                <div className="empty-state">No workspace selected</div>
            )}
        </div>
    );
}
