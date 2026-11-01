/**
 * 采购申请查询 (Query)
 */

import { gql } from '@apollo/client';
import { useLazyQuery, QueryHookOptions } from '@apollo/client';
import { handleNetworkError } from '@/utils/apollo-error';
import type {
  PurchaseRequestsQuery,
  PurchaseRequestsQueryVariables,
  PurchaseRequestQuery,
  PurchaseRequestQueryVariables,
} from '@/graphql/types/graphql';

/**
 * 采购申请列表查询
 */
export const PURCHASE_REQUESTS_QUERY = gql`
  query PurchaseRequests(
    $pagination: PagingParamInput
    $filter: PurchaseRequestFilterInput
    $sort: [FieldSortInput!]
  ) {
    purchaseRequests(pagination: $pagination, filter: $filter, sort: $sort) {
      totalCount
      nodes {
        id
        requestNo
        departmentId
        reason
        procurementType
        status
        applicantId
        createdAt
        updatedAt
        department {
          id
          name
          code
        }
        applicant {
          id
          name
        }
        details {
          id
          itemNo
          purchaseRequestId
          purchaseRequisitionFormIds
          procurementMethod
          requiredQuantity
          currentStock
          expectedDeliveryDate
          remark
          status
          purchaserId
          contractNo
          createdAt
          updatedAt
          material {
            id
            name
            specification
            unit
          }
          projects {
            id
            name
          }
          files {
            id
            name
            ext
          }
          purchaser {
            id
            name
          }
          specifiedMaterials {
            materialId
            materialName
            specification
            unit
            batch
          }
        }
      }
    }
  }
`;

/**
 * 采购申请详情查询
 */
export const PURCHASE_REQUEST_QUERY = gql`
  query PurchaseRequest($id: ID!) {
    purchaseRequest(id: $id) {
      id
      requestNo
      departmentId
      reason
      procurementType
      status
      applicantId
      createdAt
      updatedAt
      department {
        id
        name
        code
      }
      applicant {
        id
        name
      }
      details {
        id
        itemNo
        purchaseRequestId
        purchaseRequisitionFormIds
        procurementMethod
        requiredQuantity
        currentStock
        expectedDeliveryDate
        remark
        status
        purchaserId
        contractNo
        processingStatus
        createdAt
        updatedAt
        material {
          id
          name
          specification
          unit
          stock
          minLeadTime
        }
        projects {
          id
          name
        }
        files {
          id
          name
          ext
          mimeType
        }
        purchaser {
          id
          name
        }
        specifiedMaterials {
          materialId
          materialName
          specification
          unit
          batch
        }
      }
      tasks {
        assignee {
          name
        }
        title
        decided
        note
        result
        completed
      }
      process {
        todos {
          assignee {
            name
          }
          title
          decided
          note
          result
          completed
          type
        }
      }
    }
  }
`;

// ==================== 导出类型 ====================

/** 采购申请类型 */
export type PurchaseRequests = PurchaseRequestsQuery['purchaseRequests']['nodes'][0];

// ==================== Hooks ====================

/**
 * 采购申请列表查询 Hook
 */
export const usePurchaseRequests = (
  options?: QueryHookOptions<PurchaseRequestsQuery, PurchaseRequestsQueryVariables>,
) => {
  return useLazyQuery<PurchaseRequestsQuery, PurchaseRequestsQueryVariables>(
    PURCHASE_REQUESTS_QUERY,
    {
      ...options,
      onError: handleNetworkError,
    },
  );
};

/**
 * 采购申请详情查询 Hook
 */
export const usePurchaseRequest = (
  options?: QueryHookOptions<PurchaseRequestQuery, PurchaseRequestQueryVariables>,
) => {
  return useLazyQuery<PurchaseRequestQuery, PurchaseRequestQueryVariables>(PURCHASE_REQUEST_QUERY, {
    ...options,
    onError: handleNetworkError,
  });
};
