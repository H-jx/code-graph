import { describe, expect, it } from 'vitest';
import { PurchaseRequestStatus } from './purchase-request-status.vo';

describe('PurchaseRequestStatus', () => {
  it('exposes values', () => {
    expect(PurchaseRequestStatus.PENDING_SUBMIT.value).toBe('PENDING_SUBMIT');
    expect(PurchaseRequestStatus.CLOSED.label).toBe('已关闭');
  });

  it('fromValue', () => {
    expect(PurchaseRequestStatus.fromValue('APPROVED')).toBe(PurchaseRequestStatus.APPROVED);
    expect(PurchaseRequestStatus.fromValue('X')).toBeNull();
  });

  it('all returns 5', () => {
    expect(PurchaseRequestStatus.all()).toHaveLength(5);
  });

  it('equals', () => {
    expect(PurchaseRequestStatus.APPROVED.equals(PurchaseRequestStatus.APPROVED)).toBe(true);
  });

  it('toString/toJSON', () => {
    expect(PurchaseRequestStatus.DELETED.toString()).toBe('DELETED');
    expect(PurchaseRequestStatus.DELETED.toJSON()).toEqual({ value: 'DELETED', label: '已删除' });
  });
});