'use client';

import { useState, useEffect } from 'react';
import styles from './TaskPanel.module.css';

export default function TaskPanel({ tasks, isOpen, onToggle, onTaskClick }) {
  const [sortBy, setSortBy] = useState('deadline');
  const [filter, setFilter] = useState('all');

  // Load sort preference from localStorage
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const savedSort = localStorage.getItem('task-sort-by');
      if (savedSort) setSortBy(savedSort);
      
      const savedFilter = localStorage.getItem('task-filter');
      if (savedFilter) setFilter(savedFilter);
    }
  }, []);

  // Save sort preference
  const handleSortChange = (newSort) => {
    setSortBy(newSort);
    if (typeof window !== 'undefined') {
      localStorage.setItem('task-sort-by', newSort);
    }
  };

  // Save filter preference
  const handleFilterChange = (newFilter) => {
    setFilter(newFilter);
    if (typeof window !== 'undefined') {
      localStorage.setItem('task-filter', newFilter);
    }
  };

  // Filter tasks
  const filteredTasks = tasks.filter(task => {
    if (filter === 'all') return true;
    
    if (!task.deadline) return filter === 'no-deadline';
    
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    
    const taskDate = new Date(task.deadline);
    taskDate.setHours(0, 0, 0, 0);
    
    if (filter === 'overdue') {
      return taskDate < today;
    } else if (filter === 'today') {
      return taskDate.getTime() === today.getTime();
    } else if (filter === 'week') {
      const weekFromNow = new Date(today);
      weekFromNow.setDate(weekFromNow.getDate() + 7);
      return taskDate >= today && taskDate <= weekFromNow;
    } else if (filter === 'no-deadline') {
      return false;
    }
    
    return true;
  });

  // Sort tasks
  const sortedTasks = [...filteredTasks].sort((a, b) => {
    if (sortBy === 'deadline') {
      if (!a.deadline && !b.deadline) return a.file_path.localeCompare(b.file_path);
      if (!a.deadline) return 1;
      if (!b.deadline) return -1;
      return a.deadline.localeCompare(b.deadline);
    } else if (sortBy === 'file') {
      return a.file_path.localeCompare(b.file_path);
    } else if (sortBy === 'status') {
      const statusOrder = { 'Todo': 0, 'Doing': 1, 'Pending': 2 };
      return (statusOrder[a.status] || 0) - (statusOrder[b.status] || 0);
    }
    return 0;
  });

  const getStatusBadgeClass = (status) => {
    switch (status) {
      case 'Todo': return styles.statusTodo;
      case 'Doing': return styles.statusDoing;
      case 'Pending': return styles.statusPending;
      default: return styles.statusTodo;
    }
  };

  const formatDeadline = (deadline) => {
    if (!deadline) return '-';
    
    const date = new Date(deadline);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    
    const taskDate = new Date(date);
    taskDate.setHours(0, 0, 0, 0);
    
    if (taskDate < today) {
      return <span className={styles.overdue}>{deadline}</span>;
    } else if (taskDate.getTime() === today.getTime()) {
      return <span className={styles.today}>{deadline}</span>;
    }
    
    return deadline;
  };

  return (
    <div className={`${styles.taskPanel} ${isOpen ? styles.expanded : styles.collapsed}`}>
      <div className={styles.header} onClick={onToggle}>
        <div className={styles.headerLeft}>
          <span className={styles.toggleIcon}>{isOpen ? '▼' : '▲'}</span>
          <h3>Tasks ({sortedTasks.length})</h3>
        </div>
        {isOpen && (
          <div className={styles.controls} onClick={(e) => e.stopPropagation()}>
            <div className={styles.controlGroup}>
              <label>Filter:</label>
              <select value={filter} onChange={(e) => handleFilterChange(e.target.value)}>
                <option value="all">All</option>
                <option value="overdue">Overdue</option>
                <option value="today">Today</option>
                <option value="week">This Week</option>
                <option value="no-deadline">No Deadline</option>
              </select>
            </div>
            <div className={styles.controlGroup}>
              <label>Sort:</label>
              <select value={sortBy} onChange={(e) => handleSortChange(e.target.value)}>
                <option value="deadline">Deadline</option>
                <option value="file">File</option>
                <option value="status">Status</option>
              </select>
            </div>
          </div>
        )}
      </div>
      
      {isOpen && (
        <div className={styles.taskList}>
          {sortedTasks.length === 0 ? (
            <div className={styles.emptyState}>No tasks found</div>
          ) : (
            sortedTasks.map((task, idx) => (
              <div
                key={idx}
                className={styles.taskItem}
                onClick={() => onTaskClick(task.file_path, task.stable_id, task.row)}
                title={`${task.file_path} - Line ${task.row}`}
              >
                <div className={styles.fileName}>{task.file_path}</div>
                <div className={styles.taskText}>{task.line_text}</div>
                <div className={`${styles.statusBadge} ${getStatusBadgeClass(task.status)}`}>
                  {task.status}
                </div>
                <div className={styles.deadline}>{formatDeadline(task.deadline)}</div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
